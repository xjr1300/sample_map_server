use actix_web::{web, App, HttpServer};
use database::connect_to_database;
use dotenvy::dotenv;

use map_server::handlers;
use map_server::telemetries::{get_subscriber, init_subscriber};

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
            .route("/health_check", web::get().to(handlers::health_check))
            .route("/prefectures", web::get().to(handlers::prefectures))
            .route("/cities", web::get().to(handlers::cities))
            .route(
                "/tiled_cities/{zoom}/{x}/{y}",
                web::get().to(handlers::tiled_cities),
            )
            .route("/post_offices", web::get().to(handlers::post_offices))
            .route(
                "/tiled_post_offices/{zoom}/{x}/{z}",
                web::get().to(handlers::tiled_post_offices),
            )
            .app_data(pool.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
