use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

use arboard::Clipboard;
use fontes_core::{Error, Result};

/// Clipboard helper; copies run off the UI thread so Wayland tools do not block the TUI.
pub struct ClipboardStore;

impl ClipboardStore {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Queues a copy and returns immediately so the event loop keeps running.
    pub fn set_text(&self, text: &str) -> Result<()> {
        let text = text.to_owned();
        thread::spawn(move || {
            if let Err(e) = copy_blocking(&text) {
                eprintln!("fontes: clipboard: {e}");
            }
        });
        Ok(())
    }
}

fn copy_blocking(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    if use_wl_copy() {
        if copy_with_wl_copy(text).is_ok() {
            return Ok(());
        }
    }

    let mut clipboard = Clipboard::new()
        .map_err(|e| Error::Message(format!("clipboard unavailable: {e}")))?;

    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;
        clipboard
            .set()
            .wait()
            .text(text.to_owned())
            .map_err(|e| Error::Message(format!("clipboard copy failed: {e}")))?;
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    clipboard
        .set_text(text.to_owned())
        .map_err(|e| Error::Message(format!("clipboard copy failed: {e}")))
}

#[cfg(target_os = "linux")]
fn use_wl_copy() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg(not(target_os = "linux"))]
fn use_wl_copy() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn copy_with_wl_copy(text: &str) -> Result<()> {
    // Pass as argv when small enough — avoids stdin/pipe deadlocks with some compositors.
    const ARG_LIMIT: usize = 32_000;
    if text.len() <= ARG_LIMIT && !text.contains('\0') {
        let output = Command::new("wl-copy")
            .arg(text)
            .output()
            .map_err(|e| Error::Message(format!("wl-copy failed to start: {e}")))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Message(format!(
            "wl-copy failed: {}",
            stderr.trim()
        )));
    }

    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Message(format!("wl-copy failed to start: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| Error::Message(format!("wl-copy write failed: {e}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| Error::Message(format!("wl-copy wait failed: {e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(Error::Message(format!(
        "wl-copy failed: {}",
        stderr.trim()
    )))
}
