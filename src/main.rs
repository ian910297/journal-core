use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;

mod db;
mod models;
mod handlers;
mod markdown_processor;

use handlers::post_handler::{
    get_posts,
    get_post_by_uuid,
};
use handlers::asset_handler::{
    get_asset,
    get_post_assets,
};

#[get("/")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    let pool = db::create_pool();
    
    // 注意：在生產環境中，不應該每次啟動都初始化資料庫
    // 這裡僅供開發測試使用
    // db::init_db(&pool).await;

    println!("🚀 Server started successfully");
    println!("📍 Health check: http://localhost:8080/");
    println!("📚 API endpoints (Read-Only):");
    println!("   GET    /api/posts           - 取得文章列表");
    println!("   GET    /api/posts/:uuid     - 取得單一文章");
    println!("   GET    /api/assets/:uuid    - 取得資源檔案");
    println!("   GET    /api/posts/:uuid/assets - 取得文章的所有資源");
    println!();
    println!("💡 使用 CLI 進行文章管理：");
    println!("   cargo run --bin journal_cli -- add -t 'Title' -f post.md");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            // Health check
            .service(health_check)
            // Post endpoints (Read-Only)
            .service(get_posts)
            .service(get_post_by_uuid)
            // Asset endpoints
            .service(get_asset)
            .service(get_post_assets)
            // Static files (僅供開發使用，生產環境建議使用 nginx)
            .service(Files::new("/static", "static").show_files_listing())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}