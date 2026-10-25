use std::path::Path;

use anyhow::{Context, Result, ensure};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypePNG};
use objc2_foundation::{NSData, NSString, NSURL};

/// Puts the screenshot on the general pasteboard as both PNG bytes and a file URL.
///
/// Image-aware apps (browsers, Slack, Notes) take the PNG. Terminals like Ghostty only
/// paste text, but turn a file URL into its escaped path, which Claude Code attaches as
/// an image, the same as drag-and-drop.
pub fn copy_png(png: &[u8], path: &Path) -> Result<()> {
    let path = path.to_str().context("screenshot path is not UTF-8")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path))
        .absoluteString()
        .context("building file URL")?;

    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    // SAFETY: the NSPasteboardType* statics are immutable AppKit constants.
    let (png_type, url_type) = unsafe { (NSPasteboardTypePNG, NSPasteboardTypeFileURL) };
    ensure!(
        pasteboard.setData_forType(Some(&NSData::with_bytes(png)), png_type)
            && pasteboard.setString_forType(&url, url_type),
        "failed to write the screenshot to the clipboard"
    );
    Ok(())
}
