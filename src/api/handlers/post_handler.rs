use actix_web::{get, web, HttpResponse, Responder};
use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::common::models::{Post, PostResponse, Pagination};

/// 取得所有文章列表
/// GET /api/posts?page=1&limit=10
#[get("/api/posts")]
pub async fn get_posts(
    pool: web::Data<Pool>,
    pagination: web::Query<Pagination>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let offset = (pagination.page - 1) * pagination.limit;

    let rows = match client
        .query(
            "SELECT id, uuid, title, content, created_at FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            &[&(pagination.limit as i64), &(offset as i64)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let posts: Vec<PostResponse> = rows
        .into_iter()
        .map(|row| {
            let post = Post::from(row);
            PostResponse::from(post)
        })
        .collect();

    HttpResponse::Ok().json(posts)
}

/// 透過 UUID 取得單一文章
/// GET /api/posts/{uuid}
#[get("/api/posts/{uuid}")]
pub async fn get_post_by_uuid(
    pool: web::Data<Pool>,
    uuid: web::Path<Uuid>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let row = match client
        .query_one(
            "SELECT id, uuid, title, content, created_at FROM posts WHERE uuid = $1",
            &[&uuid.into_inner()],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::NotFound().body("Post not found"),
    };

    let post = Post::from(row);
    let response = PostResponse::from(post);
    
    HttpResponse::Ok().json(response)
}