use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::error::Result;

/// KJV translation id in current content bundles.
pub const KJV_TRANSLATION_ID: i64 = 1;

const USER_SCHEMA: &str = include_str!("../../../schema/user.sql");

pub(crate) fn connect(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::Error::Message(e.to_string()))?;
    }
    let conn = Connection::open(path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

pub(crate) fn init_user_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(USER_SCHEMA)?;
    Ok(())
}

/// Opens content (read-only) and user (read-write) databases.
pub struct Database {
    content: Connection,
    user: Connection,
    content_path: PathBuf,
    user_path: PathBuf,
}

impl Database {
    pub fn open(content_path: impl AsRef<Path>, user_path: impl AsRef<Path>) -> Result<Self> {
        let content_path = content_path.as_ref().to_path_buf();
        let user_path = user_path.as_ref().to_path_buf();

        let content = Connection::open_with_flags(
            &content_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        content.pragma_update(None, "foreign_keys", "ON")?;

        let user = Connection::open(&user_path)?;
        user.pragma_update(None, "foreign_keys", "ON")?;
        user.pragma_update(None, "journal_mode", "WAL")?;

        Ok(Self {
            content,
            user,
            content_path,
            user_path,
        })
    }

    pub fn open_fixture_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        Self::open(dir.join("content.sqlite"), dir.join("user.sqlite"))
    }

    /// Open databases under a data directory, creating `user.sqlite` if missing.
    pub fn open_data_dir(dir: impl AsRef<Path>) -> Result<Self> {
        use std::fs;

        use crate::paths::{content_db_path, user_db_path};

        let dir = dir.as_ref();
        fs::create_dir_all(dir).map_err(|e| crate::error::Error::Message(e.to_string()))?;
        let content = content_db_path(dir);
        let user = user_db_path(dir);
        if !content.exists() {
            return Err(crate::error::Error::Message(format!(
                "content database not found at {}\n\
                 Install one with:\n\
                   fontes sync --bundle <fontes-core-*.zip>\n\
                   fontes sync --sqlite <content.sqlite>\n\
                 Or point at a directory that already has content.sqlite:\n\
                   fontes --data-dir <dir> tui",
                content.display()
            )));
        }
        if !user.exists() {
            let conn = connect(&user)?;
            init_user_db(&conn)?;
            drop(conn);
        }
        Self::open(content, user)
    }

    pub fn content(&self) -> &Connection {
        &self.content
    }

    pub fn user(&self) -> &Connection {
        &self.user
    }

    pub fn content_path(&self) -> &Path {
        &self.content_path
    }

    pub fn user_path(&self) -> &Path {
        &self.user_path
    }

    pub fn bundle_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .content()
            .prepare("SELECT value FROM bundle_meta WHERE key = ?1")?;
        let value = stmt.query_row([key], |row| row.get(0));
        match value {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
