use pulldown_cmark::{Event, Parser, Tag};
use reqwest::Client;
use std::path::Path;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use url::Url;
use uuid::Uuid;
use futures_util::future::join_all;
use std::collections::HashMap;

const UPLOADS_DIR: &str = "static/uploads";

/// Processes markdown content to find image and link URLs,
/// downloads them, saves them locally, and replaces the URLs
/// with local paths using string replacement.
pub async fn process_markdown(content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Ensure the uploads directory exists
    fs::create_dir_all(UPLOADS_DIR).await?;

    let client = Client::new();
    let parser = Parser::new(content);

    let mut download_futures = Vec::new();
    let mut urls_to_download = std::collections::HashSet::new();

    // First pass: collect all unique remote URLs to download
    for event in parser {
        if let Event::Start(Tag::Image { dest_url, .. }) | Event::Start(Tag::Link { dest_url, .. }) = event {
            if is_remote_url(&dest_url) && urls_to_download.insert(dest_url.to_string()) {
                 let client = client.clone();
                 let url = dest_url.to_string();
                 download_futures.push(tokio::spawn(async move {
                    (url.clone(), download_and_save_file(&client, &url).await)
                }));
            }
        }
    }

    let results = join_all(download_futures).await;
    
    let mut url_map: HashMap<String, String> = HashMap::new();
    for result in results {
        match result {
            Ok((original_url, Ok(Some(new_path)))) => {
                url_map.insert(original_url, new_path);
            }
            // You might want to log errors here
            _ => (),
        }
    }

    // Second pass: replace URLs in the original content string
    let mut modified_content = content.to_string();
    for (original_url, new_path) in url_map {
        modified_content = modified_content.replace(&original_url, &new_path);
    }

    Ok(modified_content)
}

/// Checks if a URL is remote (http or https).
fn is_remote_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Downloads a file from a URL and saves it to the UPLOADS_DIR.
/// Returns the new local path if successful.
async fn download_and_save_file(client: &Client, url_str: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return Ok(None), // Ignore invalid URLs
    };

    let extension = Path::new(url.path())
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("tmp");

    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    let file_path = Path::new(UPLOADS_DIR).join(&filename);

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Ok(None);
    }

    let mut file = File::create(&file_path).await?;
    let content = response.bytes().await?;
    file.write_all(&content).await?;

    // Create a relative path for the markdown
    let local_path = format!("/{}", file_path.to_str().unwrap_or_default());
    Ok(Some(local_path))
}
