use pulldown_cmark::{Event, Parser, Tag};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use url::Url;
use uuid::Uuid;
use futures_util::future::join_all;
use std::collections::HashMap;
use sha2::{Sha256, Digest};

const UPLOADS_DIR: &str = "static/uploads";

#[derive(Debug, Clone)]
pub struct DownloadedAsset {
    pub asset_uuid: Uuid,
    pub original_url: String,
    pub file_path: String,
    pub content_type: Option<String>,
    pub file_size: i64,
}

/// 處理 markdown 內容並下載遠端資源
/// 返回處理後的 markdown 和下載的資源列表
pub async fn process_markdown(
    content: &str,
    post_id: i32,
    api_base_url: Option<&str>,
) -> Result<(String, Vec<DownloadedAsset>), Box<dyn std::error::Error + Send + Sync>> {
    // 優先使用傳入的參數，否則嘗試從環境變數讀取
    let base_url = match api_base_url {
        Some(url) => url.to_string(),
        None => std::env::var("API_BASE_URL").unwrap_or_default(),
    };
    fs::create_dir_all(UPLOADS_DIR).await?;

    let client = Client::new();
    let parser = Parser::new(content);

    let mut download_futures = Vec::new();
    let mut urls_to_download = std::collections::HashSet::new();

    // 收集所有需要下載的遠端 URL
    for event in parser {
        if let Event::Start(Tag::Image { dest_url, .. }) | Event::Start(Tag::Link { dest_url, .. }) = event {
            if is_remote_url(&dest_url) && urls_to_download.insert(dest_url.to_string()) {
                let client = client.clone();
                let url = dest_url.to_string();
                download_futures.push(tokio::spawn(async move {
                    (url.clone(), download_and_save_file(&client, &url, post_id).await)
                }));
            }
        }
    }

    // 等待所有下載任務完成
    let results = join_all(download_futures).await;
    
    let mut url_map: HashMap<String, String> = HashMap::new();
    let mut assets: Vec<DownloadedAsset> = Vec::new();
    
    for result in results {
        match result {
            Ok((original_url, Ok(Some(asset)))) => {
                // 使用完整 URL 或相對路徑
                let api_path = if base_url.is_empty() {
                    format!("/api/assets/{}", asset.asset_uuid)
                } else {
                    format!("{}/api/assets/{}", base_url, asset.asset_uuid)
                };
                url_map.insert(original_url, api_path);
                assets.push(asset);
            }
            Err(join_error) => eprintln!("Download task failed: {}", join_error),
            Ok((original_url, Err(e))) => eprintln!("Failed to download {}: {}", original_url, e),
            _ => (),
        }
    }

    // 替換 URL
    let mut modified_content = content.to_string();
    for (original_url, new_path) in url_map {
        modified_content = modified_content.replace(&original_url, &new_path);
    }

    Ok((modified_content, assets))
}

fn is_remote_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// 下載檔案並儲存到隨機目錄中
async fn download_and_save_file(
    client: &Client,
    url_str: &str,
    post_id: i32,
) -> Result<Option<DownloadedAsset>, Box<dyn std::error::Error + Send + Sync>> {
    let url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return Ok(None),
    };

    let response = client.get(url.clone()).send().await?;
    if !response.status().is_success() {
        return Ok(None);
    }

    let content_type = response.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|content_type| content_type.parse::<mime::Mime>().ok());

    let (extension, mime_str) = match content_type {
        Some(m) => {
            let ext = match (m.type_(), m.subtype()) {
                (mime::IMAGE, mime::JPEG) => Some("jpg"),
                (mime::IMAGE, mime::PNG) => Some("png"),
                (mime::IMAGE, mime::GIF) => Some("gif"),
                (mime::IMAGE, mime::SVG) => Some("svg"),
                (mime::APPLICATION, mime::PDF) => Some("pdf"),
                _ => None,
            };
            (ext, Some(m.to_string()))
        }
        None => (None, None),
    };

    if let Some(ext) = extension {
        let content = response.bytes().await?;
        let file_size = content.len() as i64;
        
        // 生成隨機目錄名稱（使用 URL 的 hash 確保同一來源的檔案在同一目錄）
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", post_id, url_str));
        let hash = format!("{:x}", hasher.finalize());
        let dir_name = &hash[..16]; // 取前 16 字元
        
        // 為檔案生成 UUID
        let asset_uuid = Uuid::new_v4();
        let filename = format!("{}.{}", asset_uuid, ext);
        
        // 建立目錄結構
        let upload_dir = PathBuf::from(UPLOADS_DIR).join(dir_name);
        fs::create_dir_all(&upload_dir).await?;
        
        let file_path = upload_dir.join(&filename);
        let mut file = File::create(&file_path).await?;
        file.write_all(&content).await?;

        // 返回相對於 UPLOADS_DIR 的路徑
        let relative_path = format!("{}/{}", dir_name, filename);
        
        Ok(Some(DownloadedAsset {
            asset_uuid,
            original_url: url_str.to_string(),
            file_path: relative_path,
            content_type: mime_str,
            file_size,
        }))
    } else {
        Ok(None)
    }
}