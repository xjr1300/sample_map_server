use std::{fs::File, io::Read, str::FromStr};

use geojson::{Feature, FeatureCollection, JsonObject};
use regex::Regex;
use serde_json::Value;

const GIFU_ADMINISTRATIVE_DIVISION_FILE: &str = "resources/N03-22_21_220101.geojson";

/// 国土交通省国土数値情報ダウンロードサイトから取得した行政区域データ(GeoJSONファイル)を読み込み。
///
/// # Returns
///
/// フィーチャーコレクション。
fn read_features() -> FeatureCollection {
    // GEOJSONファイルの内容を読み込み
    let mut file = File::open(GIFU_ADMINISTRATIVE_DIVISION_FILE).expect("file not found.");
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
fn get_epsg_code(fc: &FeatureCollection) -> u32 {
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

    captures.get(1).unwrap().as_str().parse::<u32>().unwrap()
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
        let value = get_feature_property(&f, &format!("N03_00{}", num));
        if value.is_some() {
            if value.unwrap() != "" {
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
    let name = get_feature_property(&f, "N03_001").unwrap();
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
    let area = get_feature_property(&f, "N03_003");
    let name = get_feature_property(&f, "N03_004").unwrap();
    let code = get_feature_property(&f, "N03_007").unwrap();
    let mut properties = JsonObject::new();
    properties.insert("code".to_owned(), code.into());
    properties.insert(
        "area".to_owned(),
        if area.is_some() {
            area.unwrap().into()
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
        if is_prefecture(&f) {
            prefectures.push(create_prefecture_feature(&f));
        } else {
            cities.push(create_city_feature(&f));
        }
    }

    (prefectures, cities)
}

fn main() {
    // GEOJSONファイルの内容を読み込み
    let fc = read_features();
    dbg!(fc.features.len());
    // EPSGコードを取得
    let epsg = get_epsg_code(&fc);
    dbg!(epsg);
    // 県と市町村にフィーチャーを分割
    let (pref_fs, city_fs) = divide_prefectures_and_cities(&fc);
    dbg!(pref_fs.len());
    dbg!(city_fs.len());
}
