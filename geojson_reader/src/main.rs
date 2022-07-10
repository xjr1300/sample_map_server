use std::{fs::File, io::Read, str::FromStr};

use geojson::FeatureCollection;

const GIFU_ADMINISTRATIVE_DIVISION_FILE: &str = "resources/N03-22_21_220101.geojson";

fn main() {
    // GEOJSONファイルの内容を読み込み
    let mut file = File::open(GIFU_ADMINISTRATIVE_DIVISION_FILE).expect("file not found.");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("file content is incorrect.");

    // GEOJSONファイルの内容をフィーチャコレクションに変換
    let feature_collection =
        FeatureCollection::from_str(&content).expect("geojson file is incorrect.");
    dbg!(feature_collection.features.len());
    dbg!(feature_collection.foreign_members);



}
