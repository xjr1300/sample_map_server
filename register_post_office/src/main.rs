use clap::Parser;

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

fn main() {
    println!("Hello, world!");
}
