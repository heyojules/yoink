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
    write_png(&NSPasteboard::generalPasteboard(), png, path)
}

fn write_png(pasteboard: &NSPasteboard, png: &[u8], path: &Path) -> Result<()> {
    let path = path.to_str().context("screenshot path is not UTF-8")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path))
        .absoluteString()
        .context("building file URL")?;

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

#[cfg(test)]
mod tests {
    use objc2::msg_send;

    use super::*;

    /// Uses a private pasteboard, so running tests never touches the user's clipboard.
    #[test]
    fn writes_png_bytes_and_percent_encoded_file_url() {
        let pasteboard = NSPasteboard::pasteboardWithUniqueName();
        let result = write_png(
            &pasteboard,
            b"png bytes",
            Path::new("/tmp/yoink test/shot.png"),
        );

        // SAFETY: the NSPasteboardType* statics are immutable AppKit constants.
        let (png_type, url_type) = unsafe { (NSPasteboardTypePNG, NSPasteboardTypeFileURL) };
        let png = pasteboard.dataForType(png_type).map(|d| d.to_vec());
        let url = pasteboard.stringForType(url_type).map(|s| s.to_string());
        // SAFETY: releaseGlobally takes no arguments and returns nothing; it frees the
        // unique pasteboard in the pasteboard server.
        let () = unsafe { msg_send![&pasteboard, releaseGlobally] };

        result.unwrap();
        assert_eq!(png.as_deref(), Some(&b"png bytes"[..]));
        assert_eq!(url.as_deref(), Some("file:///tmp/yoink%20test/shot.png"));
    }
}
