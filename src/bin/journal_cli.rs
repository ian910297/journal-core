use clap::{Parser, Subcommand};
use deadpool_postgres::Pool;
use dotenvy::dotenv;
use std::fs;
use std::io::{self, Read};
use std::error::Error;
use uuid::Uuid;

#[path="../db.rs"]
mod db;
#[path="../models.rs"]
mod models;
#[path="../markdown_processor.rs"]
mod markdown_processor;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add a new blog post
    Add {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        file: String,
    },
    /// List all blog posts
    List {
        #[arg(short, long, default_value_t = 1)]
        page: u32,
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
    },
    /// Get a single blog post by UUID
    Get {
        #[arg(short, long)]
        uuid: String,
    },
    /// Update an existing blog post by UUID
    Update {
        #[arg(short, long)]
        uuid: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Delete a blog post by UUID
    Delete {
        #[arg(short, long)]
        uuid: String,
    },
    /// Initialize the database
    InitDb,
    /// Test markdown processing
    TestMarkdown {
        #[arg(short, long)]
        file: String,
    },
    /// List all assets for a post
    ListAssets {
        #[arg(short, long)]
        uuid: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv().ok();
    let cli = Cli::parse();
    let pool = db::create_pool();

    match &cli.command {
        Commands::Add { title, file } => {
            let mut content = String::new();
            fs::File::open(file)?.read_to_string(&mut content)?;
            let uuid = add_post(&pool, title.clone(), content).await?;
            println!("Blog post '{}' added successfully with UUID: {}", title, uuid);
        }
        Commands::List { page, limit } => {
            list_posts(&pool, *page, *limit).await?;
        }
        Commands::Get { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            get_post(&pool, post_uuid).await?;
        }
        Commands::Update { uuid, title, file } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            let mut content: Option<String> = None;
            if let Some(f) = file {
                let mut file_content = String::new();
                fs::File::open(f)?.read_to_string(&mut file_content)?;
                content = Some(file_content);
            }
            update_post(&pool, post_uuid, title.clone(), content).await?;
            println!("Blog post {} updated successfully.", uuid);
        }
        Commands::Delete { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            delete_post(&pool, post_uuid).await?;
            println!("Blog post {} deleted successfully.", uuid);
        }
        Commands::InitDb => {
            db::init_db(&pool).await;
            println!("Database initialized successfully.");
        }
        Commands::TestMarkdown { file } => {
            let mut content = String::new();
            fs::File::open(file)?.read_to_string(&mut content)?;
            // 使用假的 post_id 進行測試
            let (processed_content, assets) = markdown_processor::process_markdown(&content, 0, None).await?;
            println!("=== Processed Content ===\n{}\n", processed_content);
            println!("=== Downloaded Assets ===");
            for asset in assets {
                println!("  - UUID: {}", asset.asset_uuid);
                println!("    Original: {}", asset.original_url);
                println!("    Path: {}", asset.file_path);
                println!("    Type: {:?}", asset.content_type);
                println!("    Size: {} bytes\n", asset.file_size);
            }
        }
        Commands::ListAssets { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            list_assets(&pool, post_uuid).await?;
        }
    }

    Ok(())
}

async fn add_post(pool: &Pool, title: String, content: String) -> Result<Uuid, Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    
    // 先建立 post 以取得 post_id
    let row = client.query_one(
        "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING id, uuid",
        &[&title, &""],
    ).await?;
    
    let post_id: i32 = row.get("id");
    let post_uuid: Uuid = row.get("uuid");
    
    // CLI 使用相對路徑（因為通常在同一伺服器）
    let (processed_content, assets) = markdown_processor::process_markdown(&content, post_id, None).await?;
    
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

async fn list_posts(pool: &Pool, page: u32, limit: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
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

async fn get_post(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let row = client
        .query_one("SELECT id, uuid, title, content, created_at FROM posts WHERE uuid = $1", &[&uuid])
        .await?;
    let post = models::Post::from(row);
    println!("ID: {}\nUUID: {}\nTitle: {}\nCreated At: {:?}\nContent:\n{}", 
             post.id, post.uuid, post.title, post.created_at, post.content);
    Ok(())
}

async fn update_post(pool: &Pool, uuid: Uuid, title: Option<String>, content: Option<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    if let Some(c) = &content {
        // 處理 markdown
        let (processed_content, assets) = markdown_processor::process_markdown(c, post_id, None).await?;
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

async fn delete_post(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let result = client.execute("DELETE FROM posts WHERE uuid = $1", &[&uuid]).await?;
    if result == 0 {
        return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, format!("Post with UUID {} not found.", uuid))));
    }
    Ok(())
}

async fn list_assets(pool: &Pool, uuid: Uuid) -> Result<(), Box<dyn Error + Send + Sync>> {
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
        let asset = models::PostAsset::from(row);
        println!("\n  Asset UUID: {}", asset.asset_uuid);
        println!("  Original URL: {}", asset.original_url);
        println!("  File Path: {}", asset.file_path);
        println!("  Content Type: {:?}", asset.content_type);
        println!("  File Size: {:?} bytes", asset.file_size);
    }
    
    Ok(())
}