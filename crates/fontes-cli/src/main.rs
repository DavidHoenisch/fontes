mod sync;

use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};
use fontes_core::{default_data_dir, Database, KJV_TRANSLATION_ID};

#[derive(Parser)]
#[command(name = "fontes", about = "Fontes Bible study")]
struct Cli {
    #[arg(long, env = "FONTES_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Interactive terminal UI.
    Tui {
        #[arg(long, default_value = "Jhn")]
        book: String,
        #[arg(long, default_value_t = 1)]
        chapter: i32,
        /// Do not restore last reading position from user.sqlite.
        #[arg(long)]
        no_resume: bool,
    },
    /// Install or update the content database from a bundle.
    Sync {
        /// Path to fontes-core-*.zip bundle.
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Install a content.sqlite file directly (skips manifest check).
        #[arg(long)]
        sqlite: Option<PathBuf>,
        /// Download a bundle zip from a URL (e.g. GitHub release asset).
        #[arg(long)]
        url: Option<String>,
    },
    /// Build and package scripture data.
    Data {
        #[command(subcommand)]
        command: DataCommand,
    },
    /// Print bundle metadata.
    Info,
    /// List Bible books in the content bundle.
    Books {
        #[arg(long, help = "Only books with verse data")]
        available: bool,
    },
    /// Print a chapter.
    Chapter {
        #[arg(long, default_value = "Jhn")]
        book: String,
        #[arg(long, default_value_t = 1)]
        chapter: i32,
    },
    /// Look up a Strong's number.
    Strong {
        key: String,
        #[arg(long, default_value_t = 5)]
        occurrences: usize,
    },
    /// Full-text search.
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum DataCommand {
    /// Build full KJV + Strong's bundle (runs tools/data/build_full.py).
    Build,
}

fn open_db(data_dir: &PathBuf) -> fontes_core::Result<Database> {
    Database::open_data_dir(data_dir)
}

fn find_build_script() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let script = dir.join("tools/data/build_full.py");
        if script.is_file() {
            return Some(script);
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

fn run_data_build() -> fontes_core::Result<()> {
    let script = find_build_script().ok_or_else(|| {
        fontes_core::Error::Message(
            "could not find tools/data/build_full.py (run from the fontes repo root)".into(),
        )
    })?;
    let status = Command::new("python3")
        .arg(&script)
        .status()
        .map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    if !status.success() {
        return Err(fontes_core::Error::Message("build_full.py failed".into()));
    }
    Ok(())
}

fn main() -> fontes_core::Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.unwrap_or_else(default_data_dir);

    match cli.command {
        CliCommand::Tui {
            book,
            chapter,
            no_resume,
        } => {
            fontes_tui::run(fontes_tui::TuiOptions {
                data_dir,
                book,
                chapter,
                resume: !no_resume,
            })?;
        }
        CliCommand::Sync {
            bundle,
            sqlite,
            url,
        } => {
            let sources = [bundle.is_some(), sqlite.is_some(), url.is_some()]
                .iter()
                .filter(|&&x| x)
                .count();
            if sources != 1 {
                eprintln!("Provide exactly one of --bundle, --sqlite, or --url");
                std::process::exit(1);
            }
            match (bundle, sqlite, url) {
                (Some(b), None, None) => sync::sync_from_bundle(&b, &data_dir)?,
                (None, Some(s), None) => sync::sync_from_sqlite(&s, &data_dir)?,
                (None, None, Some(u)) => sync::sync_from_url(&u, &data_dir)?,
                _ => unreachable!(),
            }
        }
        CliCommand::Data { command } => match command {
            DataCommand::Build => run_data_build()?,
        },
        CliCommand::Info => {
            let db = open_db(&data_dir)?;
            if let Some(v) = db.bundle_meta("bundle_version")? {
                println!("bundle_version: {v}");
            }
            if let Some(v) = db.bundle_meta("scope")? {
                println!("scope: {v}");
            }
            println!("content: {}", db.content_path().display());
            println!("user: {}", db.user_path().display());
        }
        CliCommand::Books { available } => {
            let db = open_db(&data_dir)?;
            let books = if available {
                db.list_books_with_content()?
            } else {
                db.list_books()?
            };
            for book in books {
                let max_ch = db.max_chapter(book.id).unwrap_or(0);
                println!(
                    "{:>3} {:<4} {} ({} ch)",
                    book.sort_order, book.abbrev, book.name, max_ch
                );
            }
        }
        CliCommand::Chapter { book, chapter } => {
            let db = open_db(&data_dir)?;
            let ch = db.get_chapter_kjv(&book, chapter)?;
            println!(
                "{} {} ({} verses)\n",
                ch.book.name,
                ch.chapter,
                ch.verses.len()
            );
            for v in &ch.verses {
                print!("{}:{}  ", ch.chapter, v.reference.verse);
                for t in &v.tokens {
                    if let Some(ref key) = t.strong_key {
                        print!("{}[{}] ", t.surface, key);
                    } else {
                        print!("{} ", t.surface);
                    }
                }
                println!();
            }
        }
        CliCommand::Strong { key, occurrences } => {
            let db = open_db(&data_dir)?;
            let entry = db.get_strong(&key)?;
            println!("{} ({})", entry.key, entry.lang);
            if let Some(l) = &entry.lemma {
                println!("lemma: {l}");
            }
            if let Some(t) = &entry.translit {
                println!("translit: {t}");
            }
            println!("\n{}\n", entry.definition);
            if let Some(g) = &entry.kjv_gloss {
                println!("KJV: {g}");
            }
            let total = db.count_occurrences(&key, KJV_TRANSLATION_ID)?;
            println!("\nOccurrences in bundle: {total}");
            for occ in db.list_occurrences_kjv(&key, occurrences, 0)? {
                println!(
                    "  {} {}:{} token {}",
                    occ.book_abbrev, occ.chapter, occ.verse, occ.token_idx
                );
            }
        }
        CliCommand::Search { query, limit } => {
            let db = open_db(&data_dir)?;
            for hit in db.search_verses(&query, KJV_TRANSLATION_ID, limit)? {
                println!(
                    "{} {}:{} — {}",
                    hit.book_abbrev, hit.chapter, hit.verse, hit.snippet
                );
            }
        }
    }

    Ok(())
}
