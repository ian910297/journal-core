use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    pub id: i32,
    pub uuid: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: SystemTime,
}

// API Response 結構（不包含內部 ID）
#[derive(Serialize, Debug)]
pub struct PostResponse {
    pub uuid: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: SystemTime,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        PostResponse {
            uuid: post.uuid,
            title: post.title,
            content: post.content,
            created_at: post.created_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostAsset {
    pub id: i32,
    pub post_id: i32,
    pub asset_uuid: Uuid,
    pub original_url: String,
    pub file_path: String,
    pub content_type: Option<String>,
    pub file_size: Option<i64>,
    pub created_at: SystemTime,
}

#[derive(Deserialize)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_page() -> u64 {
    1
}

fn default_limit() -> u64 {
    10
}

impl From<tokio_postgres::Row> for Post {
    fn from(row: tokio_postgres::Row) -> Self {
        Post {
            id: row.get("id"),
            uuid: row.get("uuid"),
            title: row.get("title"),
            content: row.get("content"),
            created_at: row.get("created_at"),
        }
    }
}

impl From<tokio_postgres::Row> for PostAsset {
    fn from(row: tokio_postgres::Row) -> Self {
        PostAsset {
            id: row.get("id"),
            post_id: row.get("post_id"),
            asset_uuid: row.get("asset_uuid"),
            original_url: row.get("original_url"),
            file_path: row.get("file_path"),
            content_type: row.get("content_type"),
            file_size: row.get("file_size"),
            created_at: row.get("created_at"),
        }
    }
}