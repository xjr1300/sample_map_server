use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use database::connect_to_database;
use dotenvy::dotenv;
use sqlx::PgPool;

async fn health_check() -> impl Responder {
    "Are you ready?"
}

async fn prefectures(pool: web::Data<PgPool>) -> HttpResponse {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) count FROM prefectures
        "#
    )
    .fetch_one(pool.as_ref())
    .await;
    match result {
        Ok(result) => {
            HttpResponse::Ok().body(format!("Prefecture count is {}", result.count.unwrap()))
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("{}", e)),
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let pool = web::Data::new(connect_to_database().await);

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
