use actix_web::{web, HttpResponse, Responder};
use geojson::{JsonObject, JsonValue};
use geozero::wkb;
use proj::Proj;
use slippy_map_tiles as smt;
use sqlx::{types::Uuid, PgPool};

use utils::{EPSG_WEB_MERCATOR, EPSG_WGS84};

#[tracing::instrument(name = "Health check")]
pub async fn health_check() -> impl Responder {
    "Are you ready?"
}

#[tracing::instrument(name = "Prefectures", skip(pool))]
pub async fn prefectures(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query!(
        r#"
        SELECT json_build_object(
            'type', 'FeatureCollection',
            'features', json_agg(ST_AsGeoJSON(p.*)::json)
        ) as fc
        FROM (
            SELECT id, name, geom  FROM prefectures
        ) p
        "#,
    )
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(result) => HttpResponse::Ok().json(result.fc.unwrap()),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

#[tracing::instrument(name = "Cities", skip(pool))]
pub async fn cities(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query!(
        r#"
        SELECT json_build_object(
            'type', 'FeatureCollection',
            'features', json_agg(ST_AsGeoJSON(c.*)::json)
        ) as fc
        FROM (
            SELECT id, code, area, name, geom FROM cities
        ) c
        "#,
    )
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(result) => HttpResponse::Ok().json(result.fc.unwrap()),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

struct FeatureRecord {
    feature: Option<String>,
}

async fn generate_features(records: &[FeatureRecord]) -> String {
    let mut features = "[".to_owned();
    for record in records {
        features.push_str(record.feature.as_ref().unwrap());
        features.push(',');
    }
    if 1 < features.len() {
        features.remove(features.len() - 1);
    }
    features.push(']');

    features
}

#[tracing::instrument(name = "Tiled cities", skip(pool))]
pub async fn tiled_cities(
    path: web::Path<(u8, u32, u32)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let polygon = tile_polygon(path.0, path.1, path.2)?;
    let result = sqlx::query_as!(
        FeatureRecord,
        r#"
        SELECT ST_AsGeoJSON(c.*) feature
        FROM (
            SELECT
                id, code, area, name, geom FROM cities
            WHERE
                ST_Intersects(geom, ST_GeomFromText($1, $2))
        ) c
        "#,
        polygon,
        EPSG_WEB_MERCATOR,
    )
    .fetch_all(pool.as_ref())
    .await;

    match result {
        Ok(result) => {
            let features = generate_features(&result).await;
            Ok(actix_web::HttpResponse::Ok().body(format!(
                r#"{{"features": {}, "type": "FeatureCollection"}}"#,
                features
            )))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
    }
}

#[tracing::instrument(name = "Post offices", skip(pool))]
pub async fn post_offices(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query!(
        r#"
        SELECT json_build_object(
            'type', 'FeatureCollection',
            'features', json_agg(ST_AsGeoJSON(p.*)::json)
        ) as fc
        FROM (
            SELECT id, city_code, category_code, subcategory_code, post_office_code,
            name, address, geom FROM post_offices
        ) p
        "#,
    )
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(result) => HttpResponse::Ok().json(result.fc.unwrap()),
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

struct PostOffice {
    id: Uuid,
    city_code: String,
    category_code: String,
    subcategory_code: String,
    post_office_code: String,
    name: String,
    address: String,
    geom: wkb::Decode<geo_types::Geometry<f64>>,
}

fn generate_post_office_feature(post_office: &PostOffice) -> String {
    let mut properties = JsonObject::new();
    properties.insert(
        "cityCode".to_string(),
        JsonValue::from(post_office.city_code.to_string()),
    );
    properties.insert(
        "categoryCode".to_string(),
        JsonValue::from(post_office.category_code.to_string()),
    );
    properties.insert(
        "subcategoryCode".to_string(),
        JsonValue::from(post_office.subcategory_code.to_string()),
    );
    properties.insert(
        "postOfficeCode".to_string(),
        JsonValue::from(post_office.post_office_code.to_string()),
    );
    properties.insert(
        "name".to_string(),
        JsonValue::from(post_office.name.to_string()),
    );
    properties.insert(
        "address".to_string(),
        JsonValue::from(post_office.address.to_string()),
    );
    let geometry = geojson::Value::from(post_office.geom.geometry.as_ref().unwrap());
    let feature = geojson::Feature {
        bbox: None,
        geometry: Some(geojson::Geometry {
            value: geometry,
            bbox: None,
            foreign_members: None,
        }),
        id: Some(geojson::feature::Id::String(post_office.id.to_string())),
        properties: Some(properties),
        foreign_members: None,
    };

    feature.to_string()
}

async fn generate_post_office_features(post_offices: &[PostOffice]) -> String {
    let mut features = String::from("[");
    for post_office in post_offices {
        features.push_str(&generate_post_office_feature(post_office));
        features.push(',');
    }
    if !post_offices.is_empty() {
        features.remove(features.len() - 1);
    }
    features.push(']');

    features
}

#[tracing::instrument(name = "Tiled post offices", skip(pool))]
pub async fn tiled_post_offices(
    path: web::Path<(u8, u32, u32)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let polygon = tile_polygon(path.0, path.1, path.2)?;
    let result = sqlx::query_as!(
        PostOffice,
        r#"
        SELECT
            id, city_code, category_code, subcategory_code, post_office_code,
            name, address, geom as "geom!: _"
        FROM
            post_offices
        WHERE
            ST_Intersects(geom, ST_GeomFromText($1, $2))
        "#,
        polygon,
        EPSG_WEB_MERCATOR,
    )
    .fetch_all(pool.as_ref())
    .await;

    match result {
        Ok(result) => {
            let features = generate_post_office_features(&result).await;
            Ok(HttpResponse::Ok().body(format!(
                r#"{{"features": {}, "type": "FeatureCollection"}}"#,
                features,
            )))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("{}", e))),
    }
}

fn tile_polygon(zoom: u8, x: u32, y: u32) -> Result<String, actix_web::Error> {
    let tile = smt::Tile::new(zoom, x, y);
    if tile.is_none() {
        return Err(actix_web::error::ErrorBadRequest("Invalid tile info"));
    };
    let tile = tile.unwrap();
    // タイルの範囲をWGS84緯度経度で取得
    let lb = (tile.left(), tile.bottom());
    let rt = (tile.right(), tile.top());
    // タイルの範囲をWebメルカトル座標に変換
    let from = format!("EPSG:{}", EPSG_WGS84);
    let to = format!("EPSG:{}", EPSG_WEB_MERCATOR);
    let ft_to_m = Proj::new_known_crs(&from, &to, None).unwrap();
    let mut lb = ft_to_m.convert(lb).unwrap();
    let mut rt = ft_to_m.convert(rt).unwrap();
    /*
        タイル範囲を拡張
        https://stackoverflow.com/questions/63527124/openlayers-vector-tiles-styling-features-at-edges
        If you are producing your own tiles make sure they have a buffer overlapping the adjacent tiles
        docs.mapbox.com/vector-tiles/specification/#encoding-geometry – Mike Aug 21, 2020 at 20:53
    */
    let x_expand = (rt.0 - lb.0) * 2.0 / 10.0;
    lb.0 -= x_expand;
    rt.0 += x_expand;
    let y_expand = (rt.1 - lb.1) * 2.0 / 10.0;
    lb.1 -= y_expand;
    rt.1 += y_expand;
    // タイルの範囲を示すポリゴンを定義
    Ok(format!(
        "POLYGON(({} {}, {} {}, {} {}, {} {}, {} {}))",
        lb.0, lb.1, rt.0, lb.1, rt.0, rt.1, lb.0, rt.1, lb.0, lb.1,
    ))
}
