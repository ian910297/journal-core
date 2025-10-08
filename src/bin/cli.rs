use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::error::Error;
use uuid::Uuid;

use journal_core::common::db;
use journal_core::cli::commands;

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
    
    // CLI 讀取 API_BASE_URL（用於生成完整 URL）
    let api_base_url = std::env::var("API_BASE_URL").ok();
    
    let cli = Cli::parse();
    let pool = db::create_pool();

    match &cli.command {
        Commands::Add { title, file } => {
            let uuid = commands::add_post(&pool, title, file, api_base_url.as_deref()).await?;
            println!("Blog post '{}' added successfully with UUID: {}", title, uuid);
        }
        Commands::List { page, limit } => {
            commands::list_posts(&pool, *page, *limit).await?;
        }
        Commands::Get { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            commands::get_post(&pool, post_uuid).await?;
        }
        Commands::Update { uuid, title, file } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            commands::update_post(&pool, post_uuid, title.clone(), file.clone(), api_base_url.as_deref()).await?;
            println!("Blog post {} updated successfully.", uuid);
        }
        Commands::Delete { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            commands::delete_post(&pool, post_uuid).await?;
            println!("Blog post {} deleted successfully.", uuid);
        }
        Commands::InitDb => {
            db::init_db(&pool).await;
            println!("Database initialized successfully.");
        }
        Commands::TestMarkdown { file } => {
            commands::test_markdown(file, api_base_url.as_deref()).await?;
        }
        Commands::ListAssets { uuid } => {
            let post_uuid = Uuid::parse_str(uuid)?;
            commands::list_assets(&pool, post_uuid).await?;
        }
    }

    Ok(())
}