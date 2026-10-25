<p align="center">
  <img src="assets/yoink-banner.png" alt="Yoink — a tiny macOS screenshot tool" width="800">
</p>

<p align="center">
  <strong>Grab it. Paste it. Done.</strong><br>
  A tiny macOS screenshot tool that stays out of your way.
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#use">Use</a> ·
  <a href="#under-the-hood">Under the hood</a>
</p>

---

### One hotkey. Three things.

| Select | Save | Paste |
| :--- | :--- | :--- |
| Grab a region or window with the native macOS selector. | A PNG lands in `~/Desktop/screenshots/`. | The image and its file URL are already on your clipboard. |

No uploads. No Dock icon. No menu bar. **Not memory-only:** screenshots stay on disk until you delete them.

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

Press your hotkey, select, then paste into a chat, document, or compatible terminal.

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
