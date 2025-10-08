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
            DROP TABLE IF EXISTS post_assets CASCADE;
            DROP TABLE IF EXISTS posts CASCADE;
            
            -- Posts 表，新增 uuid 欄位
            CREATE TABLE posts (
                id SERIAL PRIMARY KEY,
                uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
                title VARCHAR NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            
            -- 建立 uuid 索引以加速查詢
            CREATE INDEX idx_posts_uuid ON posts(uuid);
            
            -- Assets 映射表
            CREATE TABLE post_assets (
                id SERIAL PRIMARY KEY,
                post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
                asset_uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
                original_url TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content_type VARCHAR(100),
                file_size BIGINT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            
            -- 建立索引
            CREATE INDEX idx_post_assets_post_id ON post_assets(post_id);
            CREATE INDEX idx_post_assets_uuid ON post_assets(asset_uuid);
        ",
        )
        .await
        .expect("Failed to create database schema");
}