<p align="center">
  <img src="assets/yoink-banner.png" alt="Yoink — a tiny macOS screenshot tool" width="800">
</p>

<p align="center">
  <strong>Take a screenshot. Paste it immediately.</strong><br>
  Press your hotkey, select an area, then press ⌘V where you need it.
</p>

<p align="center">
  <a href="#why-yoink">Why Yoink?</a> ·
  <a href="#install">Install</a> ·
  <a href="#use">Use</a> ·
  <a href="#under-the-hood">Under the hood</a>
</p>

---

## Why Yoink?

You want to show something on your screen in a chat, bug report, document, or terminal-based coding assistant. Take the screenshot, switch back, and paste. No finding the file or dragging it into the app.

macOS already supports screenshots:

- **⌘⇧4** saves a file by default.
- **⌃⌘⇧4** copies an image to the clipboard instead.
- **Yoink** saves the file **and** puts both the image and its file URL on the clipboard in one capture.

Image-aware apps can paste the image. Compatible terminals can paste the saved file's path, which tools such as Claude Code can use as an image attachment. Terminal support varies.

If you only need to paste images into regular apps, the built-in **⌃⌘⇧4** may be enough. Yoink is useful when you also want a saved copy or paste screenshots into terminal tools.

Files go to `~/Desktop/screenshots/`. **Not memory-only:** they stay there until you delete them. No uploads, Dock icon, or menu bar.

## Install

**macOS** · **Rust 1.85+** (edition 2024) · **Xcode Command Line Tools**

```sh
git clone https://github.com/heyojules/yoink.git
cd yoink
make install HOTKEY=cmd+shift+2
```

Starts immediately and at login. Allow **Screen Recording** in **System Settings → Privacy & Security**, then run `make restart`.

> **Prefer ⌘⇧4?** Run `make install` without `HOTKEY`. Disable the matching macOS screenshot shortcut first. Yoink does not change system shortcuts for you.

## Use

Press your hotkey, select a region or window, then press **⌘V** in the destination app. The screenshot is already on your clipboard.

<p align="center">
  <kbd>Drag</kbd> select a region &nbsp; · &nbsp;
  <kbd>Space</kbd> select a window &nbsp; · &nbsp;
  <kbd>Esc</kbd> cancel
</p>

```sh
~/.local/bin/yoink  # capture once; print the saved path
make logs          # follow background errors
make restart       # restart the service
make uninstall     # remove the service and binary; keep screenshots
```

Re-enable any macOS shortcut you disabled after uninstalling.

## Under the hood

Rust calls macOS `screencapture -i`. AppKit puts PNG bytes and a file URL on the clipboard. A global hotkey triggers capture; launchd keeps the background service running.

<details>
<summary><strong>A note on signing</strong></summary>

Installation uses ad-hoc signing, so rebuilds may require granting Screen Recording permission again. For a stable signing identity:

```sh
make install HOTKEY=cmd+shift+2 SIGN_ID="Your signing identity"
```

</details>
