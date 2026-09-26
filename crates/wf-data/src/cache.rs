use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::{DataError, Result};
use crate::game_data::GameData;

const ITEM_CATEGORIES: &[&str] = &[
    "Warframes",
    "Primary",
    "Secondary",
    "Melee",
    "Archwing",
    "Arch-Gun",
    "Arch-Melee",
    "Sentinels",
    "SentinelWeapons",
    "Pets",
    "Arcanes",
    "Mods",
    "Resources",
    "Misc",
    "Gear",
    "Skins",
    "Sigils",
    "Fish",
    "Glyphs",
];

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheMeta {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    etag: Option<String>,
    last_modified: Option<String>,
}

pub async fn load_or_fetch(dir: &Path, client: &reqwest::Client) -> Result<GameData> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|source| DataError::CacheIo {
            path: dir.to_path_buf(),
            source,
        })?;
    let meta_path = dir.join("etag.json");
    let mut meta = read_meta(&meta_path).await;

    let mut item_bodies = Vec::with_capacity(ITEM_CATEGORIES.len());
    for category in ITEM_CATEGORIES {
        let file_name = format!("{category}.json");
        item_bodies.push(fetch_or_use_cache(dir, &file_name, client, &mut meta).await?);
    }
    let relics_body = fetch_or_use_cache(dir, "Relics.json", client, &mut meta).await?;
    let components_body = fetch_or_use_cache(dir, "Components.json", client, &mut meta).await?;

    write_meta(&meta_path, &meta).await?;

    GameData::from_json_parts(&item_bodies, &relics_body, &components_body)
}

async fn read_meta(path: &Path) -> CacheMeta {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return CacheMeta::default();
    };
    match serde_json::from_slice(&bytes) {
        Ok(meta) => meta,
        Err(error) => {
            warn!(
                path = %path.display(),
                %error,
                "Reading the game data cache index failed"
            );
            CacheMeta::default()
        }
    }
}

async fn write_meta(path: &Path, meta: &CacheMeta) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(meta).map_err(DataError::ParseCacheMeta)?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|source| DataError::CacheIo {
            path: path.to_path_buf(),
            source,
        })
}

async fn fetch_or_use_cache(
    dir: &Path,
    file_name: &str,
    client: &reqwest::Client,
    meta: &mut CacheMeta,
) -> Result<String> {
    let file_path: PathBuf = dir.join(file_name);
    match download_fresh(&file_path, file_name, client, meta).await {
        Ok(Some(body)) => Ok(body),
        Ok(None) => read_cached_file(&file_path).await,
        Err(error) => {
            warn!(
                file = file_name,
                %error,
                "Downloading the game data file failed, using the cached copy"
            );
            read_cached_file(&file_path)
                .await
                .map_err(|_| DataError::NoCacheAvailable(file_name.to_owned()))
        }
    }
}

async fn download_fresh(
    file_path: &Path,
    file_name: &str,
    client: &reqwest::Client,
    meta: &mut CacheMeta,
) -> Result<Option<String>> {
    let url = format!(
        "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/{file_name}"
    );
    let mut request = client.get(&url);
    if let Some(entry) = meta.entries.get(file_name) {
        if let Some(etag) = &entry.etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = &entry.last_modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }
    }

    let response = request
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(DataError::Network)?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    let etag = header(&response, reqwest::header::ETAG);
    let last_modified = header(&response, reqwest::header::LAST_MODIFIED);
    let body = response.text().await.map_err(DataError::Network)?;
    tokio::fs::write(file_path, &body)
        .await
        .map_err(|source| DataError::CacheIo {
            path: file_path.to_path_buf(),
            source,
        })?;
    meta.entries.insert(
        file_name.to_owned(),
        CacheEntry {
            etag,
            last_modified,
        },
    );
    Ok(Some(body))
}

fn header(response: &reqwest::Response, name: reqwest::header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn read_cached_file(path: &Path) -> Result<String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|source| DataError::CacheIo {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn corrupt_cache_index() {
        let dir = std::env::temp_dir().join("merframe-game-data-cache-index");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("etag.json");

        assert!(read_meta(&path).await.entries.is_empty());

        tokio::fs::write(&path, b"{\"entries\": ").await.unwrap();
        assert!(read_meta(&path).await.entries.is_empty());

        let written = serde_json::to_vec(&CacheMeta {
            entries: HashMap::from([(
                "Relics.json".to_owned(),
                CacheEntry {
                    etag: Some("\"relics-1\"".to_owned()),
                    last_modified: None,
                },
            )]),
        })
        .unwrap();
        tokio::fs::write(&path, written).await.unwrap();
        let meta = read_meta(&path).await;
        assert_eq!(
            meta.entries["Relics.json"].etag.as_deref(),
            Some("\"relics-1\"")
        );

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
