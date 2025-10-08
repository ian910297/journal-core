use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, http::header};
use actix_cors::Cors;
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
        // CORS 設定
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")     // 允許前端的 origin
            .allowed_origin("http://localhost:5173")     // Vite 預設 port
            .allowed_origin("http://127.0.0.1:3000")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET"])                // 只允許 GET
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .max_age(3600);                              // preflight 快取 1 小時

        App::new()
            .wrap(cors)                                   // 加入 CORS middleware
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