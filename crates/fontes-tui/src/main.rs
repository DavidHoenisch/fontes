use std::path::PathBuf;

use clap::Parser;
use fontes_core::default_data_dir;

#[derive(Parser)]
#[command(name = "fontes-tui", about = "Terminal Bible study")]
struct Cli {
    #[arg(long, env = "FONTES_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[arg(long, default_value = "Jhn")]
    book: String,

    #[arg(long, default_value_t = 1)]
    chapter: i32,
}

fn main() -> fontes_core::Result<()> {
    let cli = Cli::parse();
    fontes_tui::run(fontes_tui::TuiOptions {
        data_dir: cli.data_dir.unwrap_or_else(default_data_dir),
        book: cli.book,
        chapter: cli.chapter,
        resume: true,
    })
}
