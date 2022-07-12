use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;

use anyhow::anyhow;
use clap::Parser;
use database::connect_to_database;
use dotenvy::dotenv;
use geozero::wkb;
use proj::Transform;
use shapefile::{
    self,
    dbase::{FieldValue, Record},
    Shape,
};
use sqlx::{Postgres, Transaction};
use utils::{confirm_register, is_prefecture_code, EPSG_WEB_MERCATOR};

type ShapeReader = shapefile::Reader<BufReader<File>>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// 国土数値情報の郵便局データを記録したShapeファ&イル。
    #[clap(short, long, value_parser)]
    file: String,

    /// 郵便局データの都道府県コード。
    #[clap(short, long, value_parser)]
    code: String,

    /// 郵便局データの空間参照ID。
    #[clap(short, long, value_parser)]
    srid: i32,
}

/// 郵便局データを記録したShapeファイルを開く。
///
/// # Arguments
///
/// * `path` - 郵便局データを記録したシェイプファイル(*.shp)のパス。
///
/// # Returns
///
/// * Shapeファイルリーダー。
fn open_shape_file(path: &str) -> anyhow::Result<ShapeReader> {
    let reader = ShapeReader::from_path(path)?;
    if reader.header().shape_type != shapefile::ShapeType::Point {
        return Err(anyhow!(
            "Shapeファイルのシェイプタイプが、Pointではありません。"
        ));
    }

    Ok(reader)
}

/// 郵便局
struct PostOffice {
    /// ジオメトリ
    geom: geo_types::Geometry,
    /// 市区町村コード
    /// https://nlftp.mlit.go.jp/ksj/gml/codelist/AdminiBoundary_CD.xlsx
    city_code: String,
    /// 公共施設大分類コード
    /// https://nlftp.mlit.go.jp/ksj/gml/codelist/PubFacMaclassCd.html
    category_code: String,
    /// 公共施設小分類コード
    /// https://nlftp.mlit.go.jp/ksj/gml/codelist/PubFacMinclassCd.html
    subcategory_code: String,
    /// 郵便局分類コード
    /// https://nlftp.mlit.go.jp/ksj/gml/codelist/postOfficeCd.html
    post_office_code: String,
    /// 郵便局の正式名称
    name: String,
    /// 郵便局の市区町村名を省いた所在地
    address: String,
}

fn read_string_field(record: &Record, name: &str) -> Option<String> {
    match record.get(name).unwrap() {
        FieldValue::Character(value) => value.as_ref().cloned(),
        _ => None,
    }
}

/// ポイントシェイプを郵便局に変換する。
///
/// # Arguments
///
/// * `shape` - ポイントシェイプ。
/// * `record` - ポイントシェイプの属性。
/// * `srid` - Shapeファイルの空間参照系ID。
///
/// # Returns
///
/// 郵便局。
fn shape_to_post_office(shape: Shape, record: Record, srid: i32) -> PostOffice {
    // ジオメトリ
    let mut geom: geo_types::Geometry = geo_types::Geometry::<f64>::try_from(shape).unwrap();
    let from = format!("EPSG:{}", srid);
    let to = format!("EPSG:{}", EPSG_WEB_MERCATOR);
    geom.transform_crs_to_crs(&from, &to).unwrap();
    // 行政区域コード
    let city_code = read_string_field(&record, "P30_001").unwrap();
    // 公共施設大分類コード
    let category_code = read_string_field(&record, "P30_002").unwrap();
    // 公共施設小分類コード
    let subcategory_code = read_string_field(&record, "P30_003").unwrap();
    // 郵便局分類コード
    let post_office_code = read_string_field(&record, "P30_004").unwrap();
    // 名称
    let name = read_string_field(&record, "P30_005").unwrap();
    // 所在地
    let address = read_string_field(&record, "P30_006").unwrap();

    PostOffice {
        city_code,
        category_code,
        subcategory_code,
        post_office_code,
        name,
        address,
        geom,
    }
}

/// Shapeファイルに記録されている郵便局データを郵便局に変換する。
///
/// # Arguments
///
/// * `reader` - Shapeファイルリーダー。
/// * `srid` - Shapeファイルの空間参照系ID。
///
/// # Returns
///
/// 郵便局を格納したベクタ。
fn shapefile_to_features(reader: &mut ShapeReader, srid: i32) -> Vec<PostOffice> {
    let mut features = Vec::new();
    for result in reader.iter_shapes_and_records() {
        let (shape, record) = result.unwrap();
        features.push(shape_to_post_office(shape, record, srid));
    }

    features
}

/// 指定された都道府県の郵便局がデータベースにされているか確認する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `code` - 登録されているか確認する都道府県コード。
///
/// # Returns
///
/// 指定された都道府県の郵便局がデータベースに登録されている場合はtrue。登録されていない場合はfalse。
async fn exists_post_office(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> anyhow::Result<bool> {
    let code_like = format!("{}%", code);
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) offices FROM post_offices WHERE city_code LIKE $1
        "#,
        &code_like,
    )
    .fetch_one(tx)
    .await?;
    if 0 < result.offices.unwrap() {
        return Ok(true);
    }

    Ok(false)
}

/// 指定された都道府県コードの郵便局をデータベースから削除する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `code` - 郵便局を削除する都道府県コード。
async fn delete_post_offices(tx: &mut Transaction<'_, Postgres>, code: &str) -> anyhow::Result<()> {
    let code_like = format!("{}%", code);
    let _ = sqlx::query!(
        r#"
        DELETE FROM post_offices WHERE city_code LIKE $1
        "#,
        &code_like,
    )
    .execute(tx)
    .await?;

    Ok(())
}

/// 郵便局をデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `post_office` - 登録する郵便局。
async fn register_post_office(
    tx: &mut Transaction<'_, Postgres>,
    post_office: &PostOffice,
) -> anyhow::Result<()> {
    let _ = sqlx::query!(
        r#"
        INSERT INTO post_offices (
            id, city_code, category_code, subcategory_code, post_office_code,
            name, address, geom
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, $4, $5, $6, ST_SetSRID($7::geometry, $8) 
        )
        "#,
        post_office.city_code,
        post_office.category_code,
        post_office.subcategory_code,
        post_office.post_office_code,
        post_office.name,
        post_office.address,
        wkb::Encode(post_office.geom.clone()) as _,
        EPSG_WEB_MERCATOR,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        anyhow!(format!(
            "データベースに郵便局を登録するときにエラーが発生しました。{}",
            e
        ))
    });

    Ok(())
}

/// 郵便局をデータベースに登録する。
///
/// # Arguments
///
/// * `tx` - データベーストランザクション。
/// * `post_offices` - 登録する郵便局を格納したスライス。
async fn register_post_offices(
    tx: &mut Transaction<'_, Postgres>,
    post_offices: &[PostOffice],
) -> anyhow::Result<()> {
    for post_office in post_offices.iter() {
        register_post_office(tx, post_office).await?;
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
    if args.srid <= 0 {
        panic!("SRID({})が不正です。", args.srid);
    }

    // Shapeファイルを読み込み、郵便局を取得
    let mut reader = open_shape_file(&args.file)
        .map_err(|e| {
            panic!("{}", e);
        })
        .unwrap();
    let features = shapefile_to_features(&mut reader, args.srid);

    // データベースに接続して、トランザクションを開始
    let pool = connect_to_database().await;
    let mut tx = pool
        .begin()
        .await
        .expect("データベーストランザクションを開始できません。");

    // 指定された都道府県コードが一致する郵便局が登録されているか確認
    let exists = exists_post_office(&mut tx, &args.code).await;
    if let Err(e) = exists {
        panic!("{}", e);
    }
    if exists.unwrap() {
        // 指定された都道府県コードの郵便局が登録されている場合は、削除して登録することをユーザーに確認
        if !confirm_register(&args.code) {
            return;
        }
        // 指定された都道府県コードの郵便局を削除
        if let Err(e) = delete_post_offices(&mut tx, &args.code).await {
            panic!("{}", e);
        }
    }

    // 郵便局をデータベースに登録
    if let Err(e) = register_post_offices(&mut tx, &features).await {
        panic!("{}", e);
    }

    // トランザクションをコミット
    tx.commit()
        .await
        .expect("データベーストランザクションをコミットできませんでした。");
}

#[cfg(test)]
mod tests {
    use crate::open_shape_file;

    #[test]
    fn open_post_office_shape_file() {
        match open_shape_file("../resources/P30-13_21.shp") {
            Ok(_) => (),
            Err(e) => {
                assert!(false, "{}", e);
            }
        };
    }
}
