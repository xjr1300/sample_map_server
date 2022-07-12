use std::{convert::TryInto, fs::File, io::Read, str::FromStr};

use anyhow::anyhow;
use clap::Parser;
use database::connect_to_database;
use dotenvy::dotenv;
use geojson::{self, Feature, FeatureCollection, JsonObject};
use geozero::wkb;
use proj::Transform;
use regex::Regex;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use utils::{confirm_register, is_prefecture_code, SRID_WEB_MERCATOR};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// 国土交通省が配信する行政区域データを記録したGeoJSONファイル。
    #[clap(short, long, value_parser)]
    file: String,

    /// 行政区域データに記録されている都道府県のコード。
    ///
    /// 国土交通省が配信する行政区域データのファイル名から都道府県コードは得られるが、
    /// ファイル名が変更されることを考慮して、明示的に引数で指定する。
    #[clap(short, long, value_parser)]
    code: String,
}

/// 国土交通省国土数値情報ダウンロードサイトから取得した行政区域データ(GeoJSONファイル)を読み込み。
///
/// # Arguments
///
/// * `file`: 行政区域データ（GeoJSON）ファイルのパス。
///
/// # Returns
///
/// フィーチャーコレクション。
fn read_features(file: &str) -> FeatureCollection {
    // GEOJSONファイルの内容を読み込み
    let mut file = File::open(file).expect("file not found.");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("file content is incorrect.");

    // GEOJSONファイルの内容をフィーチャコレクションに変換
    FeatureCollection::from_str(&content).expect("geojson file is incorrect.")
}

/// フィーチャーコレクションからEPSGコードを取得する。
///
/// # Arguments
///
/// * `fc` - フィーチャコレクション。
///
/// # Returns
///
/// EPSGコード。
fn get_epsg_code(fc: &FeatureCollection) -> i32 {
    let crs = fc
        .foreign_members
        .as_ref()
        .unwrap()
        .get("crs")
        .unwrap()
        .get("properties")
        .unwrap()
        .get("name")
        .unwrap();
    let re = Regex::new(r"urn:ogc:def:crs:EPSG::(\d*)").unwrap();
    let captures = re.captures(crs.as_str().unwrap()).unwrap();

    captures.get(1).unwrap().as_str().parse::<i32>().unwrap()
}

/// フィーチャから属性を取得する。
///
/// # Arguments
///
/// * `f` - フィーチャー。
/// * `key` - 属性のキー（名前）。
///
/// # Returns
///
/// 属性の値。
fn get_feature_property(f: &Feature, key: &str) -> Option<String> {
    match f.properties.as_ref().unwrap().get(key).unwrap() {
        Value::Null => None,
        Value::Bool(_) => panic!("the Value::Bool is unexpected at a feature property value type."),
        Value::Number(_) => {
            panic!("the Value::Number is unexpected at a feature property value type.")
        }
        Value::String(value) => Some(value.clone()),
        Value::Array(_) => {
            panic!("the Value::Array is unexpected at a feature property value type.")
        }
        Value::Object(_) => {
            panic!("the Value::Object is unexpected at a feature property value type.")
        }
    }
}

/// フィーチャーが都道府県か確認する。
///
/// # Arguments
///
/// * `f` - フィーチャー。
///
/// # Returns
///
/// 都道府県の場合はtrue。市区町村の場合はfalse。
fn is_prefecture(f: &Feature) -> bool {
    for num in 2..=4 {
        let value = get_feature_property(f, &format!("N03_00{}", num));
        if let Some(value) = value {
            if !value.is_empty() {
                return false;
            }
        }
    }

    true
}

/// 行政区域データの属性を設定し直した、都道府県フィーチャーを作成する。
///
/// # Arguments
///
/// * `f` - 行政区域データの都道府県フィーチャー。
///
/// # Returns
///
/// 行政区域データの属性を設定し直した都道府県フィーチャー。
fn create_prefecture_feature(f: &Feature) -> Feature {
    let name = get_feature_property(f, "N03_001").unwrap();
    let mut properties = JsonObject::new();
    properties.insert("name".to_owned(), name.into());

    Feature {
        bbox: None,
        geometry: f.geometry.clone(),
        id: None,
        properties: Some(properties),
        foreign_members: None,
    }
}

/// 行政区域データの属性を設定し直した、 市区町村フィーチャーを作成する。
///
/// # Arguments
///
/// * `f` - 行政区域データの市区町村フィーチャー。
///
/// # Returns
///
/// 行政区域データの属性を設定し直した市区町村フィーチャー。
fn create_city_feature(f: &Feature) -> Feature {
    let area = get_feature_property(f, "N03_003");
    let name = get_feature_property(f, "N03_004").unwrap();
    let code = get_feature_property(f, "N03_007").unwrap();
    let mut properties = JsonObject::new();
    properties.insert("code".to_owned(), code.into());
    properties.insert(
        "area".to_owned(),
        if let Some(area) = area {
            area.into()
        } else {
            Value::Null
        },
    );
    properties.insert("name".to_owned(), name.into());

    Feature {
        bbox: None,
        geometry: f.geometry.clone(),
        id: None,
        properties: Some(properties),
        foreign_members: None,
    }
}

/// 行政区域データから読み込んだフィーチャーを、都道府県フィーチャと市区町村フィーチャーに分割する。
///
/// # Arguments
///
/// * `fc` - 行政区域データから読み込んだフィーチャを格納したフィーチャーコレクション。
///
/// # Returns
///
/// 都道府県フィーチャを格納したベクタと市区町村フィーチャを格納したベクタのタプル。
fn divide_prefectures_and_cities(fc: &FeatureCollection) -> (Vec<Feature>, Vec<Feature>) {
    let mut prefectures: Vec<Feature> = Vec::new();
    let mut cities: Vec<Feature> = Vec::new();
    for f in fc.features.iter() {
        if is_prefecture(f) {
            prefectures.push(create_prefecture_feature(f));
        } else {
            cities.push(create_city_feature(f));
        }
    }

    (prefectures, cities)
}

/// 指定された都道府県コードの都道府県または市区町村のデータが、データベースに登録されているか確認する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `code` - 都道府県コード。
///
/// # Returns
///
/// 当該都道府県またはその市区町村のデータがデータベースに登録されている場合はtrue。登録されていない場合はfalse。
async fn exists_prefecture(tx: &mut Transaction<'_, Postgres>, code: &str) -> anyhow::Result<bool> {
    let code_like = format!("{}%", code);
    let result = sqlx::query!(
        r#"
        SELECT p.prefs, c.cities FROM
        (SELECT COUNT(*) prefs FROM prefectures WHERE code = $1) p,
        (SELECT COUNT(*) cities FROM cities WHERE code LIKE $2) c;
        "#,
        code,
        &code_like,
    )
    .fetch_one(tx)
    .await
    .map_err(|e| {
        anyhow!(format!(
            "データベースに登録されているレコード数を確認するときにエラーが発生しました。{}",
            e
        ))
    })?;
    if 0 < result.prefs.unwrap() || 0 < result.cities.unwrap() {
        return Ok(true);
    }

    Ok(false)
}

/// 指定された都道府県コードの都道府県と市区町村をデータベースから削除する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `code` - 都道府県コード。
async fn delete_prefectures_and_cities(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> anyhow::Result<()> {
    sqlx::query!("DELETE FROM prefectures WHERE code = $1", code)
        .execute(&mut *tx)
        .await?;

    let code_like = format!("{}%", code);
    sqlx::query!("DELETE FROM cities WHERE code LIKE $1", code_like)
        .execute(&mut *tx)
        .await?;

    Ok(())
}

/// 都道府県フィーチャを、都道府県としてデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `f` - 都道府県フィーチャー。
/// * `code` - 都道府県コード。
/// * `srid` - 空間参照ID。
async fn register_prefecture(
    tx: &mut Transaction<'_, Postgres>,
    f: &Feature,
    code: &str,
    srid: i32,
) -> anyhow::Result<()> {
    let name = get_feature_property(f, "name").unwrap();
    let mut geom: geo_types::Geometry<f64> = f.geometry.clone().unwrap().value.try_into().unwrap();
    let from = format!("EPSG:{}", srid);
    let to = format!("EPSG:{}", SRID_WEB_MERCATOR);
    geom.transform_crs_to_crs(&from, &to).unwrap();

    let _ = sqlx::query!(
        r#"
            INSERT INTO prefectures (id, code, name, geom)
            VALUES(gen_random_uuid(), $1, $2, ST_SetSRID($3::geometry, $4))
        "#,
        code,
        name,
        wkb::Encode(geom) as _,
        SRID_WEB_MERCATOR,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        anyhow!(format!(
            "データベースに都道府県を登録するときにエラーが発生しました。{}",
            e
        ))
    });

    Ok(())
}

/// ベクタに格納された都道府県フィーチャを、都道府県としてデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `pref_fs` - 都道府県フィーチャーを格納したベクタ。
/// * `code` - 都道府県コード。
/// * `srid` - 空間参照ID。
async fn register_prefectures(
    tx: &mut Transaction<'_, Postgres>,
    pref_fs: &[Feature],
    code: &str,
    srid: i32,
) -> anyhow::Result<()> {
    for f in pref_fs.iter() {
        register_prefecture(tx, f, code, srid).await?;
    }

    Ok(())
}
/// 市区町村フィーチャを、市区町村としてデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `f` - 市区町村フィーチャー。
/// * `code` - 都道府県コード。
/// * `srid` - 空間参照ID。
async fn register_city(
    tx: &mut Transaction<'_, Postgres>,
    f: &Feature,
    srid: i32,
) -> anyhow::Result<()> {
    let code = get_feature_property(f, "code").unwrap();
    let area = get_feature_property(f, "area");
    let name = get_feature_property(f, "name").unwrap();
    let mut geom: geo_types::Geometry<f64> = f.geometry.clone().unwrap().value.try_into().unwrap();
    let from = format!("EPSG:{}", srid);
    let to = format!("EPSG:{}", SRID_WEB_MERCATOR);
    geom.transform_crs_to_crs(&from, &to).unwrap();

    let _ = sqlx::query!(
        r#"
            INSERT INTO cities (id, code, area, name, geom)
            VALUES(gen_random_uuid(), $1, $2, $3, ST_SetSRID($4::geometry, $5))
        "#,
        code,
        area,
        name,
        wkb::Encode(geom) as _,
        SRID_WEB_MERCATOR,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        anyhow!(format!(
            "データベースに市区町村を登録するときにエラーが発生しました。{}",
            e
        ))
    });

    Ok(())
}

/// ベクタに格納された市区町村フィーチャを、市区町村としてデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `city_fs` - 市区町村フィーチャベクタ。
/// * `srid` - 空間参照ID。
async fn register_cities(
    tx: &mut Transaction<'_, Postgres>,
    city_fs: &[Feature],
    srid: i32,
) -> anyhow::Result<()> {
    for f in city_fs.iter() {
        register_city(tx, f, srid).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // 環境変数を読み込み
    dotenv().ok();

    // コマンドライン引数を読み込み
    let args = Args::parse();
    if !is_prefecture_code(&args.code) {
        panic!("都道府県コード({})が不正です。", args.code);
    }

    // GEOJSONファイルの内容を読み込み
    let fc = read_features(&args.file);
    dbg!(fc.features.len());
    // EPSGコードを取得
    let epsg = get_epsg_code(&fc);
    dbg!(epsg);
    // 県と市区町村にフィーチャーを分割
    let (pref_fs, city_fs) = divide_prefectures_and_cities(&fc);
    dbg!(pref_fs.len());
    dbg!(city_fs.len());

    // データベースに接続して、トランザクションを開始
    let pool = connect_to_database().await;
    let mut tx = pool
        .begin()
        .await
        .expect("データベーストランザクションを開始できません。");

    // 指定された都道府県コードの都道府県と市区町村が登録されているか確認
    let exists = exists_prefecture(&mut tx, &args.code).await;
    if let Err(e) = exists {
        panic!("{}", e);
    }
    if exists.unwrap() {
        // 指定された都道府県コードの都道府県と市区町村が登録されている場合は、削除して登録することをユーザーに確認
        if !confirm_register(&args.code) {
            return;
        }
        // 指定された都道府県コードの都道府県と市区町村を削除
        if let Err(e) = delete_prefectures_and_cities(&mut tx, &args.code).await {
            panic!("{}", e);
        }
    }

    // 都道府県を登録
    if let Err(e) = register_prefectures(&mut tx, &pref_fs, &args.code, epsg).await {
        panic!("{}", e);
    };
    // 市区町村を登録
    if let Err(e) = register_cities(&mut tx, &city_fs, epsg).await {
        panic!("{}", e);
    };

    // トランザクションをコミット
    tx.commit()
        .await
        .expect("データベーストランザクションをコミットできませんでした。");
}
