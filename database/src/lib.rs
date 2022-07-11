use sqlx::{postgres::PgPoolOptions, PgPool};

/// 環境変数DATABASE_URLの値を使用して、データベースに接続する。
///
/// # Returns
///
/// データベースコネクションプール。
pub async fn connect_to_database() -> PgPool {
    let key = "DATABASE_URL";
    let url = std::env::var(key).unwrap_or_else(|_| {
        format!(
            "環境変数にデータベースへの接続URLを示す{}が設定されていません。",
            key
        )
    });

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("データベースに接続できません。環境変数DATABASE_URLの値を確認してください。")
}
