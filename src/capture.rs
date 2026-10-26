use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

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

    let Some(status) = run_selector(&path)? else {
        eprintln!("yoink: another screenshot selector is open; skipped");
        return Ok(None);
    };
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

/// Runs `screencapture -i` (Space toggles window mode, Esc cancels). Returns `None` if
/// macOS keeps refusing because another interactive capture is on screen, which also
/// happens briefly while the previous one closes; that's not worth reporting as a failure.
fn run_selector(path: &Path) -> Result<Option<ExitStatus>> {
    for _ in 0..4 {
        let output = Command::new("/usr/sbin/screencapture")
            .arg("-i")
            .arg(path)
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|child| child.wait_with_output())
            .context("running screencapture")?;
        if !is_busy(&output.stderr) {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            return Ok(Some(output.status));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Ok(None)
}

fn is_busy(stderr: &[u8]) -> bool {
    String::from_utf8_lossy(stderr).contains("cannot run two interactive screen captures")
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
    fn recognizes_busy_selector_message() {
        // Verbatim from screencapture on macOS 26.
        assert!(is_busy(
            b"screencapture: cannot run two interactive screen captures at a time\n"
        ));
        assert!(!is_busy(b"could not create image from rect\n"));
        assert!(!is_busy(b""));
    }

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
