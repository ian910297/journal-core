use deadpool_postgres::Pool;
use std::error::Error;
use std::fs;
use std::io::{self, Read};
use uuid::Uuid;

use crate::common::models::{Post, PostAsset};
use crate::cli::markdown_processor;

pub async fn add_post(
    pool: &Pool,
    title: &str,
    file_path: &str,
    api_base_url: Option<&str>,
) -> Result<Uuid, Box<dyn Error + Send + Sync>> {
    let mut content = String::new();
    fs::File::open(file_path)?.read_to_string(&mut content)?;
    
    let client = pool.get().await?;
    
    // 先建立 post 以取得 post_id
    let row = client.query_one(
        "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING id, uuid",
        &[&title, &""],
    ).await?;
    
    let post_id: i32 = row.get("id");
    let post_uuid: Uuid = row.get("uuid");
    
    // CLI 使用完整 URL（如果有設定）
    let (processed_content, assets) = markdown_processor::process_markdown(&content, post_id, api_base_url).await?;
    
    // 更新 post 的內容
    client.execute(
        "UPDATE posts SET content = $1 WHERE id = $2",
        &[&processed_content, &post_id],
    ).await?;
    
    // 儲存 assets 資訊到資料庫
    for asset in assets {
        client.execute(
            "INSERT INTO post_assets (post_id, asset_uuid, original_url, file_path, content_type, file_size) 
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[&post_id, &asset.asset_uuid, &asset.original_url, &asset.file_path, 
              &asset.content_type, &asset.file_size],
        ).await?;
    }
    
    Ok(post_uuid)
}

pub async fn list_posts(pool: &Pool, page: u32, limit: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let offset = (page - 1) * limit;
    let rows = client
        .query(
            "SELECT id, uuid, title, created_at FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)],
        )
        .await?;

    for row in rows {
        let id: i32 = row.get("id");
        let uuid: Uuid = row.get("uuid");
        let title: String = row.get("title");
        let created_at: std::time::SystemTime = row.get("created_at");
        println!("ID: {}, UUID: {}, Title: {}, Created At: {:?}", id, uuid, title, created_at);
    }
    Ok(())
}

pub async fn get_post(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let row = client
        .query_one("SELECT id, uuid, title, content, created_at FROM posts WHERE uuid = $1", &[&uuid])
        .await?;
    let post = Post::from(row);
    println!("ID: {}\nUUID: {}\nTitle: {}\nCreated At: {:?}\nContent:\n{}", 
             post.id, post.uuid, post.title, post.created_at, post.content);
    Ok(())
}

pub async fn update_post(
    pool: &Pool,
    uuid: Uuid,
    title: Option<String>,
    file: Option<String>,
    api_base_url: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    
    // 先取得 post_id
    let row = client.query_one("SELECT id FROM posts WHERE uuid = $1", &[&uuid]).await?;
    let post_id: i32 = row.get("id");
    
    let mut updates = Vec::new();
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    let mut owned_strings: Vec<String> = Vec::new();
    let mut param_idx = 1;

    if let Some(t) = &title {
        updates.push(format!("title = ${}", param_idx));
        params.push(t);
        param_idx += 1;
    }

    if let Some(f) = &file {
        let mut content = String::new();
        fs::File::open(f)?.read_to_string(&mut content)?;
        
        // 處理 markdown
        let (processed_content, assets) = markdown_processor::process_markdown(&content, post_id, api_base_url).await?;
        owned_strings.push(processed_content);
        updates.push(format!("content = ${}", param_idx));
        params.push(owned_strings.last().unwrap());
        param_idx += 1;
        
        // 刪除舊的 assets 記錄
        client.execute("DELETE FROM post_assets WHERE post_id = $1", &[&post_id]).await?;
        
        // 新增新的 assets
        for asset in assets {
            client.execute(
                "INSERT INTO post_assets (post_id, asset_uuid, original_url, file_path, content_type, file_size) 
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[&post_id, &asset.asset_uuid, &asset.original_url, &asset.file_path, 
                  &asset.content_type, &asset.file_size],
            ).await?;
        }
    }

    if updates.is_empty() {
        println!("No updates provided for post UUID {}.", uuid);
        return Ok(());
    }

    let query = format!(
        "UPDATE posts SET {} WHERE uuid = ${}",
        updates.join(", "),
        param_idx
    );
    params.push(&uuid);

    client.execute(&query, &params.as_slice()).await?;
    Ok(())
}

pub async fn delete_post(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let result = client.execute("DELETE FROM posts WHERE uuid = $1", &[&uuid]).await?;
    if result == 0 {
        return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, format!("Post with UUID {} not found.", uuid))));
    }
    Ok(())
}

pub async fn list_assets(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    
    // 先取得 post_id
    let row = client.query_one("SELECT id FROM posts WHERE uuid = $1", &[&uuid]).await?;
    let post_id: i32 = row.get("id");
    
    let rows = client.query(
        "SELECT * FROM post_assets WHERE post_id = $1 ORDER BY created_at",
        &[&post_id],
    ).await?;
    
    println!("Assets for post {}:", uuid);
    for row in rows {
        let asset = PostAsset::from(row);
        println!("\n  Asset UUID: {}", asset.asset_uuid);
        println!("  Original URL: {}", asset.original_url);
        println!("  File Path: {}", asset.file_path);
        println!("  Content Type: {:?}", asset.content_type);
        println!("  File Size: {:?} bytes", asset.file_size);
    }
    
    Ok(())
}

pub async fn test_markdown(file_path: &str, api_base_url: Option<&str>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut content = String::new();
    fs::File::open(file_path)?.read_to_string(&mut content)?;
    
    // 使用假的 post_id 進行測試
    let (processed_content, assets) = markdown_processor::process_markdown(&content, 0, api_base_url).await?;
    
    println!("=== Processed Content ===\n{}\n", processed_content);
    println!("=== Downloaded Assets ===");
    for asset in assets {
        println!("  - UUID: {}", asset.asset_uuid);
        println!("    Original: {}", asset.original_url);
        println!("    Path: {}", asset.file_path);
        println!("    Type: {:?}", asset.content_type);
        println!("    Size: {} bytes\n", asset.file_size);
    }
    
    Ok(())
}