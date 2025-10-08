use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use deadpool_postgres::Pool;
use uuid::Uuid;
use crate::models::{Post, PostResponse, Pagination, CreatePost};
use crate::markdown_processor;

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
            "SELECT uuid, title, content, created_at FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
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

/// 新增文章
/// POST /api/posts
/// Body: { "title": "標題", "content": "markdown 內容" }
#[post("/api/posts")]
pub async fn create_post(
    pool: web::Data<Pool>,
    post_data: web::Json<CreatePost>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // 先建立 post 以取得 post_id
    let row = match client
        .query_one(
            "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING id, uuid",
            &[&post_data.title, &""],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to create post"),
    };

    let post_id: i32 = row.get("id");
    let post_uuid: Uuid = row.get("uuid");

    // 處理 markdown 並下載資源
    let (processed_content, assets) = match markdown_processor::process_markdown(&post_data.content, post_id).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to process markdown: {}", e);
            return HttpResponse::InternalServerError().body("Failed to process markdown");
        }
    };

    // 更新 post 的內容
    if let Err(_) = client
        .execute(
            "UPDATE posts SET content = $1 WHERE id = $2",
            &[&processed_content, &post_id],
        )
        .await
    {
        return HttpResponse::InternalServerError().body("Failed to update post content");
    }

    // 儲存 assets 資訊到資料庫
    for asset in assets {
        if let Err(_) = client
            .execute(
                "INSERT INTO post_assets (post_id, asset_uuid, original_url, file_path, content_type, file_size) 
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &post_id,
                    &asset.asset_uuid,
                    &asset.original_url,
                    &asset.file_path,
                    &asset.content_type,
                    &asset.file_size,
                ],
            )
            .await
        {
            eprintln!("Failed to save asset: {}", asset.asset_uuid);
        }
    }

    HttpResponse::Created().json(serde_json::json!({
        "uuid": post_uuid,
        "message": "Post created successfully"
    }))
}

/// 更新文章
/// PUT /api/posts/{uuid}
/// Body: { "title": "新標題", "content": "新內容" } (兩者都是可選的)
#[put("/api/posts/{uuid}")]
pub async fn update_post(
    pool: web::Data<Pool>,
    uuid: web::Path<Uuid>,
    post_data: web::Json<serde_json::Value>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let post_uuid = uuid.into_inner();

    // 取得 post_id
    let row = match client
        .query_one("SELECT id FROM posts WHERE uuid = $1", &[&post_uuid])
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::NotFound().body("Post not found"),
    };

    let post_id: i32 = row.get("id");

    let title = post_data.get("title").and_then(|v| v.as_str());
    let content = post_data.get("content").and_then(|v| v.as_str());

    // 處理標題更新
    if let Some(t) = title {
        if let Err(_) = client
            .execute(
                "UPDATE posts SET title = $1 WHERE id = $2",
                &[&t, &post_id],
            )
            .await
        {
            return HttpResponse::InternalServerError().body("Failed to update title");
        }
    }

    // 處理內容更新
    if let Some(c) = content {
        // 處理 markdown
        let (processed_content, assets) = match markdown_processor::process_markdown(c, post_id).await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to process markdown: {}", e);
                return HttpResponse::InternalServerError().body("Failed to process markdown");
            }
        };

        // 更新內容
        if let Err(_) = client
            .execute(
                "UPDATE posts SET content = $1 WHERE id = $2",
                &[&processed_content, &post_id],
            )
            .await
        {
            return HttpResponse::InternalServerError().body("Failed to update content");
        }

        // 刪除舊的 assets 記錄
        let _ = client
            .execute("DELETE FROM post_assets WHERE post_id = $1", &[&post_id])
            .await;

        // 新增新的 assets
        for asset in assets {
            let _ = client
                .execute(
                    "INSERT INTO post_assets (post_id, asset_uuid, original_url, file_path, content_type, file_size) 
                     VALUES ($1, $2, $3, $4, $5, $6)",
                    &[
                        &post_id,
                        &asset.asset_uuid,
                        &asset.original_url,
                        &asset.file_path,
                        &asset.content_type,
                        &asset.file_size,
                    ],
                )
                .await;
        }
    }

    if title.is_none() && content.is_none() {
        return HttpResponse::BadRequest().body("No fields to update");
    }

    HttpResponse::Ok().json(serde_json::json!({
        "uuid": post_uuid,
        "message": "Post updated successfully"
    }))
}

/// 刪除文章
/// DELETE /api/posts/{uuid}
#[delete("/api/posts/{uuid}")]
pub async fn delete_post(
    pool: web::Data<Pool>,
    uuid: web::Path<Uuid>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let result = match client
        .execute("DELETE FROM posts WHERE uuid = $1", &[&uuid.into_inner()])
        .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if result == 0 {
        return HttpResponse::NotFound().body("Post not found");
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Post deleted successfully"
    }))
}