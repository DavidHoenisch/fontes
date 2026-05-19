//! SQLite data layer for the fontes Bible study application.

mod bible;
mod db;
mod error;
mod model;
mod notes;
mod paths;
mod search;
mod strongs;

pub use db::{Database, KJV_TRANSLATION_ID};
pub use error::{Error, Result};
pub use model::*;
pub use paths::{content_db_path, default_data_dir, user_db_path};
pub use search::search_terms;
