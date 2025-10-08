use actix_web::{get, web, HttpResponse, HttpRequest, Responder};
use deadpool_postgres::Pool;
use uuid::Uuid;
use actix_files::NamedFile;
use std::path::PathBuf;

const UPLOADS_DIR: &str = "static/uploads";

/// 透過 asset UUID 取得檔案
/// GET /api/assets/{uuid}
#[get("/api/assets/{uuid}")]
pub async fn get_asset(
    pool: web::Data<Pool>,
    uuid: web::Path<Uuid>,
    req: HttpRequest,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish().into(),
    };

    // 從資料庫查詢 asset 資訊
    let row = match client
        .query_one(
            "SELECT file_path, content_type FROM post_assets WHERE asset_uuid = $1",
            &[&uuid.into_inner()],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::NotFound().body("Asset not found").into(),
    };

    let file_path: String = row.get("file_path");
    let content_type: Option<String> = row.get("content_type");

    // 建構完整的檔案路徑
    let full_path = PathBuf::from(UPLOADS_DIR).join(&file_path);

    // 檢查檔案是否存在
    if !full_path.exists() {
        return HttpResponse::NotFound().body("File not found on disk").into();
    }

    // 返回檔案
    match NamedFile::open(&full_path) {
        Ok(mut file) => {
            // 如果有 content_type，設定它
            if let Some(ct) = content_type {
                file = file.set_content_type(
                    ct.parse::<mime::Mime>()
                        .unwrap_or(mime::APPLICATION_OCTET_STREAM)
                );
            }
            file.into_response(&req)
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to read file").into(),
    }
}

/// 取得特定 post 的所有 assets（可選功能）
/// GET /api/posts/{uuid}/assets
#[get("/api/posts/{uuid}/assets")]
pub async fn get_post_assets(
    pool: web::Data<Pool>,
    uuid: web::Path<Uuid>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // 先取得 post_id
    let post_row = match client
        .query_one("SELECT id FROM posts WHERE uuid = $1", &[&uuid.into_inner()])
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::NotFound().body("Post not found"),
    };

    let post_id: i32 = post_row.get("id");

    // 取得所有 assets
    let rows = match client
        .query(
            "SELECT asset_uuid, original_url, content_type, file_size, created_at 
             FROM post_assets 
             WHERE post_id = $1 
             ORDER BY created_at",
            &[&post_id],
        )
        .await
    {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let assets: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "asset_uuid": row.get::<_, Uuid>("asset_uuid"),
                "original_url": row.get::<_, String>("original_url"),
                "content_type": row.get::<_, Option<String>>("content_type"),
                "file_size": row.get::<_, Option<i64>>("file_size"),
                "url": format!("/api/assets/{}", row.get::<_, Uuid>("asset_uuid")),
            })
        })
        .collect();

    HttpResponse::Ok().json(assets)
}