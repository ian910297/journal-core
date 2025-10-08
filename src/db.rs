use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

pub fn create_pool() -> Pool {
    let mut cfg = Config::new();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    cfg.url = Some(db_url);
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create pool")
}

pub async fn init_db(pool: &Pool) {
    let client = pool.get().await.expect("Failed to get client from pool");
    client
        .batch_execute(
            "
            DROP TABLE IF EXISTS posts;
            CREATE TABLE posts (
                id SERIAL PRIMARY KEY,
                title VARCHAR NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        ",
        )
        .await
        .expect("Failed to create posts table");
}
