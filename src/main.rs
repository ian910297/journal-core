use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Server started successfully");

    HttpServer::new(move || {
        App::new()
            .service(health_check)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
