use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use anyhow::anyhow;
use clap::Parser;
use dbase::FieldType;
use dotenvy::dotenv;
use once_cell::sync::Lazy;

type ShapeReader = geozero_shp::Reader<BufReader<File>>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// 国土数値情報の郵便局データを記録したShapeファイル。
    #[clap(short, long, value_parser)]
    file: String,

    /// 郵便局データの空間参照ID。
    #[clap(short, long, value_parser)]
    srid: i32,
}

/// 郵便局データフィールド
static POST_OFFICE_FIELDS: Lazy<HashMap<String, FieldType>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("P30_001".to_string(), FieldType::Character);
    m.insert("P30_002".to_string(), FieldType::Character);
    m.insert("P30_003".to_string(), FieldType::Character);
    m.insert("P30_004".to_string(), FieldType::Character);
    m.insert("P30_005".to_string(), FieldType::Character);
    m.insert("P30_006".to_string(), FieldType::Character);
    m.insert("P30_007".to_string(), FieldType::Numeric);

    m
});

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
    let reader = geozero_shp::Reader::from_path(path)?;
    if reader.header().shape_type != geozero_shp::ShapeType::Point {
        return Err(anyhow!(
            "Shapeファイルのシェイプタイプが、Pointではありません。"
        ));
    }
    for field in reader.dbf_fields().unwrap().iter() {
        // フィールドが存在するか確認
        let field_name = field.name();
        let expected_field_type = POST_OFFICE_FIELDS.get(field_name);
        if expected_field_type.is_none() {
            return Err(anyhow!(
                "郵便番号データに含まれないフィールド({})が存在します。",
                field_name
            ));
        }
        let expected_field_type = *expected_field_type.unwrap();
        // フィールドのデータ型が正しいか確認
        let field_type = field.field_type();
        if field.field_type() as i32 != expected_field_type as i32 {
            return Err(anyhow!(
                "郵便番号データの{}フィールド({})が、{}型です。",
                field_name,
                expected_field_type,
                field_type,
            ));
        }
    }

    Ok(reader)
}

fn main() {
    // 環境変数を読み込み
    dotenv().ok();

    // コマンドライン引数を読み込み
    let args = Args::parse();
    if args.srid <= 0 {
        panic!("SRID({})が不正です。", args.srid);
    }

    // Shapeファイルを読み込み
    let reader = open_shape_file(&args.file)
        .map_err(|e| {
            panic!("{}", e);
        })
        .unwrap();
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
