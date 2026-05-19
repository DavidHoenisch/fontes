mod app;
mod input;
mod list_scroll;
mod markdown;
mod overlay;
mod scroll;
mod search_highlight;
mod ui;

use std::io::{self, stdout, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use fontes_core::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub use app::App;

#[derive(Debug, Clone)]
pub struct TuiOptions {
    pub data_dir: PathBuf,
    pub book: String,
    pub chapter: i32,
    /// Restore last reading position from user.sqlite when true.
    pub resume: bool,
}

struct TerminalGuard {
    stdout: Stdout,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        Ok(Self { stdout })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = self.stdout.execute(LeaveAlternateScreen);
    }
}

pub fn run(options: TuiOptions) -> Result<()> {
    let mut app = App::open(
        options.data_dir,
        &options.book,
        options.chapter,
        options.resume,
    )?;
    app.status = "Ready.".into();

    let _guard = TerminalGuard::enter().map_err(|e| fontes_core::Error::Message(e.to_string()))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
        .map_err(|e| fontes_core::Error::Message(e.to_string()))?;

    loop {
        app.tick_search()?;

        terminal
            .draw(|f| ui::draw(f, &mut app))
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?;

        let poll_ms = if app.mode == app::Mode::Search {
            50
        } else {
            100
        };
        if event::poll(Duration::from_millis(poll_ms))
            .map_err(|e| fontes_core::Error::Message(e.to_string()))?
        {
            if let Event::Key(key) =
                event::read().map_err(|e| fontes_core::Error::Message(e.to_string()))?
            {
                if input::handle_key(&mut app, key)? {
                    break;
                }
                app.tick_search()?;
            }
        }
    }

    Ok(())
}
