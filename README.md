# Yoink

A tiny macOS screenshot tool. Select a region, then paste it anywhere that accepts images.

- Copies PNG bytes and a file URL to your clipboard.
- Saves a copy in `~/Desktop/screenshots/` — not memory-only.
- Runs in the background with a global hotkey. No Dock icon, menu bar, or uploads.

## Install

Requires macOS, Rust 1.85+ (edition 2024), and Xcode Command Line Tools.

```sh
git clone https://github.com/heyojules/yoink.git
cd yoink
make install HOTKEY=cmd+shift+2
```

Starts now and at login. Allow **Screen Recording** in System Settings → Privacy & Security, then run `make restart`.

Without `HOTKEY`, the default is `cmd+shift+4`. Disable the matching macOS screenshot shortcut before using it; Yoink does not change system shortcuts.

## Use

Press your hotkey. **Drag** to select, **Space** for a window, **Esc** to cancel. Paste into a chat, document, or compatible terminal.

```sh
~/.local/bin/yoink  # capture once; print the saved path
make logs          # follow background errors
make restart       # restart the service
make uninstall     # remove the service and binary; keep screenshots
```

Re-enable any macOS shortcut you disabled after uninstalling.

## How it works

Rust calls macOS `screencapture -i`. AppKit writes the image and file URL to the clipboard. A global hotkey triggers capture; launchd keeps the service running.

Installation uses ad-hoc signing, so rebuilds may require granting permission again. For stable signing, pass `SIGN_ID="Your signing identity"` to `make install`.
