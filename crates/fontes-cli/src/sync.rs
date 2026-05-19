use std::fs::{self, File};
use std::io::{copy, Read, Write};
use std::path::{Path, PathBuf};

use fontes_core::{content_db_path, Result};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

#[derive(serde::Deserialize)]
struct BundleManifest {
    content_db_sha256: String,
    version: Option<String>,
}

pub fn sync_from_bundle(bundle_path: &Path, data_dir: &Path) -> Result<()> {
    fs::create_dir_all(data_dir).map_err(|e| fontes_core::Error::Message(e.to_string()))?;

    let file = File::open(bundle_path)
        .map_err(|e| fontes_core::Error::Message(format!("open bundle: {e}")))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| fontes_core::Error::Message(format!("read zip: {e}")))?;

    let mut manifest: Option<BundleManifest> = None;
    let temp_dir = data_dir.join(".sync-tmp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    }
    fs::create_dir_all(&temp_dir).map_err(|e| fontes_core::Error::Message(e.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
        let name = entry.name().to_string();
        let out_path = temp_dir.join(Path::new(&name).file_name().unwrap_or_default());
        if name.ends_with('/') {
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| fontes_core::Error::Message(e.to_string()))?;
        }
        let mut out = File::create(&out_path)
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
        copy(&mut entry, &mut out).map_err(|e| fontes_core::Error::Message(e.to_string()))?;

        if name.ends_with("manifest.json") {
            let text = fs::read_to_string(&out_path)
                .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
            manifest = Some(
                serde_json::from_str(&text)
                    .map_err(|e| fontes_core::Error::Message(e.to_string()))?,
            );
        }
    }

    let content_tmp = temp_dir.join("content.sqlite");
    if !content_tmp.exists() {
        return Err(fontes_core::Error::Message(
            "bundle missing content.sqlite".into(),
        ));
    }

    if let Some(ref m) = manifest {
        verify_sha256(&content_tmp, &m.content_db_sha256)?;
    }

    let dest = content_db_path(data_dir);
    let backup = data_dir.join("content.sqlite.bak");
    if dest.exists() {
        fs::rename(&dest, &backup).map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    }
    if fs::rename(&content_tmp, &dest).is_err() {
        let _ = fs::rename(&backup, &dest);
        return Err(fontes_core::Error::Message(
            "failed to install content.sqlite".into(),
        ));
    }
    let _ = fs::remove_file(&backup);

    if let Some(manifest_path) = find_file(&temp_dir, "manifest.json") {
        fs::copy(manifest_path, data_dir.join("manifest.json"))
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    }

    fs::remove_dir_all(&temp_dir).ok();

    let _ = fontes_core::Database::open_data_dir(data_dir)?;

    let version = manifest
        .and_then(|m| m.version)
        .unwrap_or_else(|| "unknown".into());
    println!("Synced content bundle v{version} → {}", dest.display());
    Ok(())
}

pub fn sync_from_url(url: &str, data_dir: &Path) -> Result<()> {
    fs::create_dir_all(data_dir).map_err(|e| fontes_core::Error::Message(e.to_string()))?;

    let tmp = data_dir.join(".download-bundle.zip");
    println!("Downloading {url}…");

    let response = reqwest::blocking::get(url)
        .map_err(|e| fontes_core::Error::Message(format!("download failed: {e}")))?;
    if !response.status().is_success() {
        return Err(fontes_core::Error::Message(format!(
            "download failed: HTTP {}",
            response.status()
        )));
    }

    let mut file = File::create(&tmp)
        .map_err(|e| fontes_core::Error::Message(format!("create temp file: {e}")))?;
    let mut reader = response;
    std::io::copy(&mut reader, &mut file)
        .map_err(|e| fontes_core::Error::Message(format!("write download: {e}")))?;
    file.flush()
        .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    drop(file);

    sync_from_bundle(&tmp, data_dir)?;
    fs::remove_file(&tmp).ok();
    Ok(())
}

pub fn sync_from_sqlite(source: &Path, data_dir: &Path) -> Result<()> {
    fs::create_dir_all(data_dir).map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    let dest = content_db_path(data_dir);
    fs::copy(source, &dest).map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    println!("Installed {} → {}", source.display(), dest.display());
    let _ = fontes_core::Database::open_data_dir(data_dir)?;
    Ok(())
}

fn verify_sha256(path: &Path, expected_hex: &str) -> Result<()> {
    let mut file = File::open(path).map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hex::encode(hasher.finalize());
    if digest != expected_hex.to_lowercase() {
        return Err(fontes_core::Error::Message(format!(
            "checksum mismatch: expected {expected_hex}, got {digest}"
        )));
    }
    Ok(())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.file_name().is_some_and(|n| n == name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        }
    }
    None
}
