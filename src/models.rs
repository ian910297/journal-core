use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: SystemTime,
}

#[derive(Deserialize)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
}

// This is a helper struct for converting from a tokio_postgres::Row
impl From<tokio_postgres::Row> for Post {
    fn from(row: tokio_postgres::Row) -> Self {
        Post {
            id: row.get("id"),
            title: row.get("title"),
            content: row.get("content"),
            created_at: row.get("created_at"),
        }
    }
}
