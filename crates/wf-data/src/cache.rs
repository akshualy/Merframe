use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures_util::future::{join5, try_join_all, try_join3};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::dictionary::Dictionary;
use crate::error::{DataError, Result};
use crate::game_data::GameData;
use crate::nightwave::NightwaveActs;
use crate::syndicate::parse_syndicates;
use crate::vosfor_pack::parse_vosfor_packs;

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

const RELICS_FILE: &str = "Relics.json";
const COMPONENTS_FILE: &str = "Components.json";
const NIGHTWAVE_ACTS_FILE: &str = "ExportNightwave.json";
const SYNDICATES_FILE: &str = "ExportSyndicates.json";
const BOOSTER_PACKS_FILE: &str = "ExportBoosterPacks.json";
const VENDORS_FILE: &str = "ExportVendors.json";
const DICTIONARY_FILE: &str = "dict.en.json";

const WFCD_ITEMS: &str = "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json";
const PUBLIC_EXPORT_PLUS: &str =
    "https://raw.githubusercontent.com/calamity-inc/warframe-public-export-plus/senpai";

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheMeta {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    etag: Option<String>,
    last_modified: Option<String>,
}

type FetchedFile = (String, Option<CacheEntry>);

impl CacheMeta {
    fn record(&mut self, file_name: &str, fresh: Option<CacheEntry>) {
        if let Some(entry) = fresh {
            self.entries.insert(file_name.to_owned(), entry);
        }
    }
}

pub async fn load_or_fetch(dir: &Path, client: &reqwest::Client) -> Result<GameData> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(cache_io(dir))?;
    let meta_path = dir.join("etag.json");
    let mut meta = read_meta(&meta_path).await;

    let item_files: Vec<String> = ITEM_CATEGORIES
        .iter()
        .map(|category| format!("{category}.json"))
        .collect();
    let (items, (relics_body, relics_entry), (components_body, components_entry)) = try_join3(
        try_join_all(
            item_files
                .iter()
                .map(|file_name| fetch_or_use_cache(dir, WFCD_ITEMS, file_name, client, &meta)),
        ),
        fetch_or_use_cache(dir, WFCD_ITEMS, RELICS_FILE, client, &meta),
        fetch_or_use_cache(dir, WFCD_ITEMS, COMPONENTS_FILE, client, &meta),
    )
    .await?;

    let mut item_bodies = Vec::with_capacity(items.len());
    for (file_name, (body, entry)) in item_files.iter().zip(items) {
        item_bodies.push(body);
        meta.record(file_name, entry);
    }
    meta.record(RELICS_FILE, relics_entry);
    meta.record(COMPONENTS_FILE, components_entry);

    let (dictionary, nightwave_acts, syndicates, booster_packs, vendors) = join5(
        fetch_or_use_cache(dir, PUBLIC_EXPORT_PLUS, DICTIONARY_FILE, client, &meta),
        fetch_or_use_cache(dir, PUBLIC_EXPORT_PLUS, NIGHTWAVE_ACTS_FILE, client, &meta),
        fetch_or_use_cache(dir, PUBLIC_EXPORT_PLUS, SYNDICATES_FILE, client, &meta),
        fetch_or_use_cache(dir, PUBLIC_EXPORT_PLUS, BOOSTER_PACKS_FILE, client, &meta),
        fetch_or_use_cache(dir, PUBLIC_EXPORT_PLUS, VENDORS_FILE, client, &meta),
    )
    .await;
    let dictionary = parsed_or_default([(DICTIONARY_FILE, dictionary)], &mut meta, |[body]| {
        Dictionary::from_json(body)
    });

    let nightwave_acts = parsed_or_default(
        [(NIGHTWAVE_ACTS_FILE, nightwave_acts)],
        &mut meta,
        |[body]| NightwaveActs::from_json(body, &dictionary),
    );
    let syndicates = parsed_or_default([(SYNDICATES_FILE, syndicates)], &mut meta, |[body]| {
        parse_syndicates(body, &dictionary)
    });
    let vosfor_packs = parsed_or_default(
        [(BOOSTER_PACKS_FILE, booster_packs), (VENDORS_FILE, vendors)],
        &mut meta,
        |[booster_packs, vendors]| parse_vosfor_packs(booster_packs, vendors, &dictionary),
    );

    write_meta(&meta_path, &meta).await?;

    Ok(
        GameData::from_json_parts(&item_bodies, &relics_body, &components_body)?
            .with_nightwave_acts(nightwave_acts)
            .with_syndicates(syndicates)
            .with_vosfor_packs(vosfor_packs),
    )
}

fn parsed_or_default<T: Default, const N: usize>(
    fetched: [(&str, Result<FetchedFile>); N],
    meta: &mut CacheMeta,
    parse: impl FnOnce([&str; N]) -> Result<T>,
) -> T {
    let file_names = fetched.each_ref().map(|(file_name, _)| *file_name);
    let files: Result<Vec<FetchedFile>> = fetched.into_iter().map(|(_, file)| file).collect();
    let parsed = files.and_then(|files| {
        let bodies = std::array::from_fn(|file| files[file].0.as_str());
        Ok((parse(bodies)?, files))
    });
    match parsed {
        Ok((parsed, files)) => {
            for (file_name, (_, entry)) in file_names.into_iter().zip(files) {
                meta.record(file_name, entry);
            }
            parsed
        }
        Err(error) => {
            warn!(
                files = file_names.join(", "),
                %error,
                "Loading the game data failed, everything read from these files is missing"
            );
            T::default()
        }
    }
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
    tokio::fs::write(path, bytes).await.map_err(cache_io(path))
}

async fn fetch_or_use_cache(
    dir: &Path,
    source: &str,
    file_name: &str,
    client: &reqwest::Client,
    meta: &CacheMeta,
) -> Result<FetchedFile> {
    let file_path: PathBuf = dir.join(file_name);
    match download_fresh(&file_path, source, file_name, client, meta).await {
        Ok(Some((body, entry))) => Ok((body, Some(entry))),
        Ok(None) => Ok((read_cached_file(&file_path).await?, None)),
        Err(error) => {
            warn!(
                file = file_name,
                %error,
                "Downloading the game data file failed, using the cached copy"
            );
            read_cached_file(&file_path)
                .await
                .map(|body| (body, None))
                .map_err(|_| DataError::NoCacheAvailable(file_name.to_owned()))
        }
    }
}

async fn download_fresh(
    file_path: &Path,
    source: &str,
    file_name: &str,
    client: &reqwest::Client,
    meta: &CacheMeta,
) -> Result<Option<(String, CacheEntry)>> {
    let mut request = client.get(format!("{source}/{file_name}"));
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
        .map_err(cache_io(file_path))?;
    Ok(Some((
        body,
        CacheEntry {
            etag,
            last_modified,
        },
    )))
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
        .map_err(cache_io(path))
}

fn cache_io(path: &Path) -> impl FnOnce(std::io::Error) -> DataError + '_ {
    |source| DataError::CacheIo {
        path: path.to_path_buf(),
        source,
    }
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
