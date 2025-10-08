use pulldown_cmark::{Event, Parser, Tag};
use reqwest::Client;
use std::path::PathBuf;
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
    // 確保 uploads 目錄存在
    fs::create_dir_all(UPLOADS_DIR).await?;

    let client = Client::new();
    let parser = Parser::new(content);

    let mut download_futures = Vec::new();
    let mut urls_to_download = std::collections::HashSet::new();

    // First pass: collect all unique remote URLs to download
    for event in parser {
        // 蒐集圖片和連結的 URL
        if let Event::Start(Tag::Image { dest_url, .. }) | Event::Start(Tag::Link { dest_url, .. }) = event {
            // 確保是遠端 URL 且是唯一的新 URL
            if is_remote_url(&dest_url) && urls_to_download.insert(dest_url.to_string()) {
                 let client = client.clone();
                 let url = dest_url.to_string();
                 // 為每個 URL 建立一個非同步下載任務
                 download_futures.push(tokio::spawn(async move {
                    (url.clone(), download_and_save_file(&client, &url).await)
                }));
            }
        }
    }

    // 等待所有下載任務完成
    let results = join_all(download_futures).await;
    
    let mut url_map: HashMap<String, String> = HashMap::new();
    for result in results {
        match result {
            // 成功下載且儲存
            Ok((original_url, Ok(Some(new_path)))) => {
                url_map.insert(original_url, new_path);
            }
            // 下載或儲存失敗，或 URL 無效，記錄並忽略
            Err(join_error) => eprintln!("Download task failed: {}", join_error),
            Ok((original_url, Err(e))) => eprintln!("Failed to download {}: {}", original_url, e),
            _ => (),
        }
    }

    // Second pass: **直接替換字串** (最高效且最符合現狀的解決方案)
    let mut modified_content = content.to_string();
    for (original_url, new_path) in url_map {
        // 將原來的遠端 URL 替換為本地相對路徑
        modified_content = modified_content.replace(&original_url, &new_path);
    }

    Ok(modified_content)
}

/// Checks if a URL is remote (http or https).
fn is_remote_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Downloads a file from a URL and saves it to the UPLOADS_DIR if it's a recognized type (image/pdf).
/// Returns the new local path if successful, otherwise returns None.
async fn download_and_save_file(client: &Client, url_str: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return Ok(None), // Ignore invalid URLs
    };

    let response = client.get(url.clone()).send().await?;
    if !response.status().is_success() {
        // Don't treat this as an error, just skip downloading
        return Ok(None);
    }

    // --- Stricter extension logic ---
    let content_type = response.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|content_type| content_type.parse::<mime::Mime>().ok());

    let extension = match content_type {
        Some(m) => match (m.type_(), m.subtype()) {
            (mime::IMAGE, mime::JPEG) => Some("jpg"),
            (mime::IMAGE, mime::PNG) => Some("png"),
            (mime::IMAGE, mime::GIF) => Some("gif"),
            (mime::IMAGE, mime::SVG) => Some("svg"),
            (mime::APPLICATION, mime::PDF) => Some("pdf"),
            _ => None, // Not a downloadable type, so we won't save it
        },
        None => None, // No content type, don't download
    };

    if let Some(ext) = extension {
        // Only proceed to download and save if the extension is recognized
        let filename = format!("{}.{}", Uuid::new_v4(), ext);
        let file_path = PathBuf::from(UPLOADS_DIR).join(&filename);

        let mut file = File::create(&file_path).await?;
        let content = response.bytes().await?;
        file.write_all(&content).await?;

        let local_path = format!("/{}", file_path.to_str().unwrap_or_default());
        Ok(Some(local_path))
    } else {
        // If the content type is not recognized, return None.
        // This ensures the original URL is kept.
        Ok(None)
    }
}
