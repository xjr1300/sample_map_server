# Sample Map Server

## 空間参照系

本アプリで扱う空間データの空間参照系は、Webメルカトル（EPSG:3857）で、この投影法の座標
でデータベースに蓄積する。

## 国土数値情報

[国土数値情報ダウンロードサイトコンテンツ利用規約](https://nlftp.mlit.go.jp/ksj/other/agreement.html)

* [行政区域データ](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-N03-v3_1.html)
  * `resources/gifu_prefecture-20220101.geojson`
* [郵便局データ](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-P30.html)
  * `resources/gifu_post_offices.shp`

## SQLx

バージョン0.5

`geozero = "0.9"`が、`sqlx = "0.6"`に対応していない。

```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```

## Proj

バージョン0.27

行政区域データなどをWebメルカトルに変換するために`proj`を使用する。
本アプリが使用する`proj-0.27`クレートは、`libproj v9.0.x`に依存している。

```bash
brew install proj
```

## PostgreSQL with PostGISコンテナの作成

```bash
docker-compose up -d
```

## PostgreSQL with PostGISコンテナの起動とデータベースマイグレーションの実行

`PostgreSQL with PostGIS`コンテナが起動していない状態で実行する。

```bash
./scripts/run_containers.sh
```

## 行政区域データの登録

```bash
cargo run --package register_prefecture -- --file ./resources/gifu_prefecture-20220101.geojson --code 21
```

## 郵便局データの登録

```bash
cargo run --package register_post_office -- --file ./resources/gifu_post_offices.shp --code 21 --srid 4612 --encoding shift_jis
```

## 郵便局地図APIサーバーの起動

```bash
cargo run --package map_server
```

## 郵便局地図の閲覧

[Sample Map App](https://github.com/xjr1300/sample_map_app)で郵便局地図を閲覧する。
