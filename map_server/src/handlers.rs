use actix_web::{web, HttpResponse, Responder};
use proj::Proj;
use slippy_map_tiles as smt;
use sqlx::PgPool;

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
        features.push_str(&record.feature.as_ref().unwrap());
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

#[tracing::instrument(name = "Tiled post offices", skip(pool))]
pub async fn tiled_post_offices(
    path: web::Path<(u8, u32, u32)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let polygon = tile_polygon(path.0, path.1, path.2)?;
    let result = sqlx::query!(
        r#"
        SELECT json_build_object(
            'type', 'FeatureCollection',
            'features', json_agg(ST_AsGeoJSON(p.*)::json)
        ) as fc
        FROM (
            SELECT
                id, city_code, category_code, subcategory_code, post_office_code,
                name, address, geom
            FROM
                post_offices
            WHERE
                ST_Intersects(geom, ST_GeomFromText($1, $2))
        ) p
        "#,
        polygon,
        EPSG_WEB_MERCATOR,
    )
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(result) => Ok(HttpResponse::Ok().json(result.fc.unwrap())),
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
    let lb = ft_to_m.convert(lb).unwrap();
    let rt = ft_to_m.convert(rt).unwrap();

    // タイルの範囲を示すポリゴンを定義
    Ok(format!(
        "POLYGON(({} {}, {} {}, {} {}, {} {}, {} {}))",
        lb.0, lb.1, rt.0, lb.1, rt.0, rt.1, lb.0, rt.1, lb.0, lb.1,
    ))
}
