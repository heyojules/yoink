use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use chrono::Local;

use crate::clipboard;

/// Runs the native macOS region selector, saves the PNG, and copies it to the clipboard.
/// Returns `None` when the user cancels the selection (Esc).
pub fn capture() -> Result<Option<PathBuf>> {
    let dir = screenshots_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let stem = Local::now()
        .format("screenshot-%Y-%m-%d-%H.%M.%S")
        .to_string();
    let path = unique_path(&dir, &stem);

    // -i: interactive selection (Space toggles window mode, Esc cancels).
    let status = Command::new("/usr/sbin/screencapture")
        .arg("-i")
        .arg(&path)
        .status()
        .context("running screencapture")?;
    if !status.success() {
        bail!("screencapture exited with {status}");
    }

    // screencapture exits successfully without writing a file when the selection is cancelled.
    let png = match std::fs::read(&path) {
        Ok(png) => png,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    // The file is already saved; name it so a clipboard failure doesn't hide where it went.
    clipboard::copy_png(&png, &path)
        .with_context(|| format!("copying {} to the clipboard", path.display()))?;
    Ok(Some(path))
}

fn screenshots_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(Path::new(&home).join("Desktop/screenshots"))
}

/// `<stem>.png`, or `<stem>-2.png`, `-3`… if two captures land in the same second.
/// No spaces, so terminals paste the path unescaped.
fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}.png"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.png"));
        n += 1;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_adds_counter_on_collision() {
        let dir = std::env::temp_dir().join(format!("yoink-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(unique_path(&dir, "shot"), dir.join("shot.png"));
        std::fs::write(dir.join("shot.png"), b"").unwrap();
        assert_eq!(unique_path(&dir, "shot"), dir.join("shot-2.png"));
        std::fs::write(dir.join("shot-2.png"), b"").unwrap();
        assert_eq!(unique_path(&dir, "shot"), dir.join("shot-3.png"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
