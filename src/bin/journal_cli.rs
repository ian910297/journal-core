use clap::{Parser, Subcommand};
use deadpool_postgres::Pool;
use dotenvy::dotenv;
use std::fs;
use std::io::{self, Read};
use std::error::Error;

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
    /// Get a single blog post by ID
    Get {
        #[arg(short, long)]
        id: i32,
    },
    /// Update an existing blog post
    Update {
        #[arg(short, long)]
        id: i32,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Delete a blog post by ID
    Delete {
        #[arg(short, long)]
        id: i32,
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
            add_post(&pool, title.clone(), content).await?;
            println!("Blog post '{}' added successfully.", title);
        }
        Commands::List { page, limit } => {
            list_posts(&pool, *page, *limit).await?;
        }
        Commands::Get { id } => {
            get_post(&pool, *id).await?;
        }
        Commands::Update { id, title, file } => {
            let mut content: Option<String> = None;
            if let Some(f) = file {
                let mut file_content = String::new();
                fs::File::open(f)?.read_to_string(&mut file_content)?;
                content = Some(file_content);
            }
            update_post(&pool, *id, title.clone(), content).await?;
            println!("Blog post with ID {} updated successfully.", id);
        }
        Commands::Delete { id } => {
            delete_post(&pool, *id).await?;
            println!("Blog post with ID {} deleted successfully.", id);
        }
    }

    Ok(())
}

async fn add_post(pool: &Pool, title: String, content: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let processed_content = markdown_processor::process_markdown(&content).await?;
    let client = pool.get().await?;
    client.execute(
        "INSERT INTO posts (title, content) VALUES ($1, $2)",
        &[&title, &processed_content],
    ).await?;
    Ok(())
}

async fn list_posts(pool: &Pool, page: u32, limit: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let offset = (page - 1) * limit;
    let rows = client
        .query(
            "SELECT id, title, created_at FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)],
        )
        .await?;

    for row in rows {
        let post = models::Post::from(row);
        println!("ID: {}, Title: {}, Created At: {:?}", post.id, post.title, post.created_at);
    }
    Ok(())
}

async fn get_post(pool: &Pool, id: i32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let row = client
        .query_one("SELECT id, title, content, created_at FROM posts WHERE id = $1", &[&id])
        .await?;
    let post = models::Post::from(row);
    println!("ID: {}\nTitle: {}\nCreated At: {:?}\nContent:\n{}", post.id, post.title, post.created_at, post.content);
    Ok(())
}

async fn update_post(pool: &Pool, id: i32, title: Option<String>, content: Option<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    
    let mut updates = Vec::new();
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    let mut owned_content_strings: Vec<String> = Vec::new(); // To hold processed content
    let mut param_idx = 1;

    if let Some(t) = &title {
        updates.push(format!("title = ${}", param_idx));
        params.push(t);
        param_idx += 1;
    }

    if let Some(c) = &content {
        let processed_content = markdown_processor::process_markdown(c).await?;
        owned_content_strings.push(processed_content); // Store the owned String
        updates.push(format!("content = ${}", param_idx));
        params.push(owned_content_strings.last().unwrap()); // Push reference to the owned String
        param_idx += 1;
    }

    if updates.is_empty() {
        println!("No updates provided for post ID {}.", id);
        return Ok(());
    }

    let query = format!(
        "UPDATE posts SET {} WHERE id = ${}",
        updates.join(", "),
        param_idx
    );
    params.push(&id);

    client.execute(&query, &params.as_slice()).await?;
    Ok(())
}

async fn delete_post(pool: &Pool, id: i32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = pool.get().await?;
    let result = client.execute("DELETE FROM posts WHERE id = $1", &[&id]).await?;
    if result == 0 {
        return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, format!("Post with ID {} not found.", id))));
    }
    Ok(())
}
