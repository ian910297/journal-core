use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;

mod db;
mod models;
mod handlers;
mod markdown_processor;

use handlers::post_handler::{create_post, get_posts, delete_post};

#[get("/")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    let pool = db::create_pool();
    db::init_db(&pool).await;

    println!("🚀 Server started successfully");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(create_post)
            .service(get_posts)
            .service(delete_post)
            .service(Files::new("/static", "static").show_files_listing())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
