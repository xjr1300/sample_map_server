use std::{fs::File, io::Read, str::FromStr};

use geojson::FeatureCollection;
use regex::Regex;

const GIFU_ADMINISTRATIVE_DIVISION_FILE: &str = "resources/N03-22_21_220101.geojson";

fn read_features() -> FeatureCollection {
    // GEOJSONファイルの内容を読み込み
    let mut file = File::open(GIFU_ADMINISTRATIVE_DIVISION_FILE).expect("file not found.");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("file content is incorrect.");

    // GEOJSONファイルの内容をフィーチャコレクションに変換
    FeatureCollection::from_str(&content).expect("geojson file is incorrect.")
}

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

fn main() {
    // GEOJSONファイルの内容を読み込み
    let fc = read_features();
    dbg!(fc.features.len());

    // EPSGコードを取得
    let epsg = get_epsg_code(&fc);
    dbg!(epsg);
}
