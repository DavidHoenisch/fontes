use std::path::PathBuf;

/// Default user data directory (`~/.local/share/fontes` on Linux).
pub fn default_data_dir() -> PathBuf {
    if let Some(base) = dirs::data_local_dir() {
        base.join("fontes")
    } else {
        PathBuf::from(".fontes")
    }
}

pub fn content_db_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("content.sqlite")
}

pub fn user_db_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("user.sqlite")
}
