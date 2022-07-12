use actix_web::{web, HttpResponse, Responder};
use sqlx::PgPool;

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

// TODO: タイル座標を受け取りそのタイルと重なる市区町村を返却するように改修
#[tracing::instrument(name = "Cities", skip(pool))]
pub async fn cities(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query!(
        r#"
        SELECT json_build_object(
            'type', 'FeatureCollection',
            'features', json_agg(ST_AsGeoJSON(c.*)::json)
        ) as fc
        FROM (
            SELECT id, code, area, name, geom  FROM cities
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

// TODO: タイル座標を受け取りそのタイルと重なる郵便局を返却するように改修
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
            name, address, geom  FROM post_offices
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
