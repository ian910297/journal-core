use actix_web::{get, web, HttpResponse, Responder};
use deadpool_postgres::Pool;
use crate::models::{Post, Pagination};

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

#[get("/api/posts/{id}")]
pub async fn get_post_by_id(pool: web::Data<Pool>, id: web::Path<i32>) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let row = match client
        .query_one("SELECT * FROM posts WHERE id = $1", &[&id.into_inner()])
        .await
    {
        Ok(row) => row,
        Err(e) => {
            if let Some(db_err) = e.as_db_error() {
                if db_err.code() == &tokio_postgres::error::SqlState::NO_DATA {
                    return HttpResponse::NotFound().finish();
                }
            }
            // For any other error, return a generic server error
            return HttpResponse::InternalServerError().finish();
        }
    };

    let post = Post::from(row);
    HttpResponse::Ok().json(post)
}
