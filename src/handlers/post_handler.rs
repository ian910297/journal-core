use actix_web::{get, post, delete, web, HttpResponse, Responder};
use deadpool_postgres::Pool;
use crate::models::{CreatePost, Post, Pagination};
use crate::markdown_processor;

#[get("/api/posts")]
pub async fn get_posts(pool: web::Data<Pool>, pagination: web::Query<Pagination>) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let offset = (pagination.page - 1) * pagination.limit;

    let rows = match client
        .query(
            "SELECT * FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            &[&(pagination.limit as i64), &(offset as i64)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let posts: Vec<Post> = rows.into_iter().map(Post::from).collect();
    HttpResponse::Ok().json(posts)
}

#[post("/api/posts")]
pub async fn create_post(pool: web::Data<Pool>, new_post: web::Json<CreatePost>) -> impl Responder {
    let processed_content = match markdown_processor::process_markdown(&new_post.content).await {
        Ok(content) => content,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to process markdown"),
    };

    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let row = match client
        .query_one(
            "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING *",
            &[&new_post.title, &processed_content],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let post = Post::from(row);
    HttpResponse::Created().json(post)
}

#[delete("/api/posts/{id}")]
pub async fn delete_post(pool: web::Data<Pool>, id: web::Path<i32>) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result = match client
        .execute("DELETE FROM posts WHERE id = $1", &[&id.into_inner()])
        .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if result == 0 {
        HttpResponse::NotFound().finish()
    } else {
        HttpResponse::NoContent().finish()
    }
}
