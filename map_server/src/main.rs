use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use database::connect_to_database;
use dotenvy::dotenv;
use map_server::telemetries::{get_subscriber, init_subscriber};
use sqlx::PgPool;

#[tracing::instrument(name = "Health check")]
async fn health_check() -> impl Responder {
    "Are you ready?"
}

#[tracing::instrument(name = "Prefectures", skip(pool))]
async fn prefectures(pool: web::Data<PgPool>) -> HttpResponse {
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let subscriber = get_subscriber("sample_map_server".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    tracing::info!("データベースと接続");
    let pool = web::Data::new(connect_to_database().await);

    tracing::info!("Webサーバーを起動");
    HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/prefectures", web::get().to(prefectures))
            .app_data(pool.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
