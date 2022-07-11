# Sample Map Server

## Docker

```bash
docker-compose up -d
```

## SQLx

```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```

## 空間参照形

本アプリで扱う空間データの空間参照系は、Webメルカトル（EPSG:3857）で、この投影法の座標
でデータベースに蓄積する。

## 国土数値情報

* [行政区域データ](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-N03-v3_1.html)
* [郵便局データ](https://nlftp.mlit.go.jp/ksj/gml/datalist/KsjTmplt-P30.html)
