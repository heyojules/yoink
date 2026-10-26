use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, anyhow};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

use crate::capture;
use crate::pop::{self, Pop};

/// Registers a global hotkey and runs the AppKit event loop, which delivers it. Never returns.
pub fn run(hotkey: &str) -> Result<()> {
    let hotkey: HotKey = hotkey
        .parse()
        .map_err(|e| anyhow!("invalid hotkey {hotkey:?}: {e}"))?;

    let mtm = MainThreadMarker::new().context("daemon must run on the main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    // No Dock icon, no menu bar, but still allowed to show the capture pop.
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Must outlive the event loop; dropping it unregisters the hotkey.
    let manager = GlobalHotKeyManager::new().context("creating hotkey manager")?;
    manager
        .register(hotkey)
        .context("registering hotkey (already taken?)")?;

    GlobalHotKeyEvent::set_event_handler(Some(on_hotkey));
    eprintln!("yoink: listening on {hotkey}");
    ensure_screen_recording_access();
    app.run();
    Ok(())
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Without Screen Recording access, captures fail with "could not create image from rect".
/// Requesting it lists yoink in System Settings → Privacy & Security → Screen Recording.
fn ensure_screen_recording_access() {
    // SAFETY: both take no arguments and only query or prompt for TCC access.
    if unsafe { CGPreflightScreenCaptureAccess() } {
        return;
    }
    unsafe { CGRequestScreenCaptureAccess() };
    eprintln!(
        "yoink: no Screen Recording access; allow yoink in System Settings, then `make restart`"
    );
}

/// Set while a selection is on screen, so repeated presses don't stack selectors.
static CAPTURING: AtomicBool = AtomicBool::new(false);

fn on_hotkey(event: GlobalHotKeyEvent) {
    if event.state != HotKeyState::Pressed || CAPTURING.swap(true, Ordering::AcqRel) {
        return;
    }
    // Carbon delivers hotkeys on the main thread; clear any pop so it isn't in the way.
    if let Some(mtm) = MainThreadMarker::new() {
        pop::dismiss(mtm);
    }
    // Off the main thread so the event loop stays responsive while the user selects.
    std::thread::spawn(|| {
        match capture::capture() {
            Ok(Some(path)) => pop::show(Pop::Saved(path)),
            Ok(None) => {}
            Err(e) => {
                eprintln!("yoink: {e:#}");
                pop::show(Pop::Failed);
            }
        }
        CAPTURING.store(false, Ordering::Release);
    });
}
