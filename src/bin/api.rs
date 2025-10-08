use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;
use journal_core::common::db;
use journal_core::api::handlers::{post_handler, asset_handler};

#[get("/")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    let pool = db::create_pool();

    println!("🚀 Server started successfully");
    println!("📍 Health check: http://localhost:8080/");
    println!("📚 API endpoints (Read-Only):");
    println!("   GET    /api/posts           - 取得文章列表");
    println!("   GET    /api/posts/:uuid     - 取得單一文章");
    println!("   GET    /api/assets/:uuid    - 取得資源檔案");
    println!("   GET    /api/posts/:uuid/assets - 取得文章的所有資源");
    println!();
    println!("💡 使用 CLI 進行文章管理：");
    println!("   cargo run --bin cli -- add -t 'Title' -f post.md");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(post_handler::get_posts)
            .service(post_handler::get_post_by_uuid)
            .service(asset_handler::get_asset)
            .service(asset_handler::get_post_assets)
            .service(Files::new("/static", "static").show_files_listing())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}