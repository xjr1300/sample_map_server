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
