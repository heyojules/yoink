//! yoink — select a screen region, save it to ~/Desktop/screenshots, and copy it to the clipboard.

mod capture;
mod clipboard;
mod daemon;

use anyhow::{Result, bail};

const DEFAULT_HOTKEY: &str = "cmd+shift+4";

const USAGE: &str = "\
usage:
  yoink                 select a region, save it, copy it to the clipboard
  yoink daemon [HOTKEY]  run in the background and capture on HOTKEY (default: cmd+shift+4)";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        [] => {
            if let Some(path) = capture::capture()? {
                println!("{}", path.display());
            }
            Ok(())
        }
        ["daemon"] => daemon::run(DEFAULT_HOTKEY),
        ["daemon", hotkey] => daemon::run(hotkey),
        ["-h" | "--help"] => {
            println!("{USAGE}");
            Ok(())
        }
        _ => bail!("unknown arguments\n{USAGE}"),
    }
}
