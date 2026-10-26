//! The "yoink pop": after a hotkey capture, a thumbnail springs in at the cursor, a green
//! check draws itself, then the card is yanked away. A failed capture shows a red badge
//! that shakes. All motion is Core Animation keyframes; Reduce Motion gets a plain fade.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::Duration;

use dispatch2::{DispatchQueue, DispatchTime};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSImage, NSPanel, NSScreen, NSStatusWindowLevel, NSView,
    NSWindowAnimationBehavior, NSWindowCollectionBehavior, NSWindowSharingType, NSWindowStyleMask,
    NSWorkspace,
};
use objc2_core_graphics::{CGColor, CGMutablePath, CGPath};
use objc2_foundation::{NSArray, NSNumber, NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::{
    CAKeyframeAnimation, CALayer, CAMediaTiming, CAMediaTimingFunction, CAShapeLayer,
    kCAFillModeBoth, kCAGravityResizeAspectFill, kCALineCapRound, kCALineJoinRound,
    kCAMediaTimingFunctionEaseInEaseOut,
};

pub enum Pop {
    Saved(PathBuf),
    Failed,
}

/// Shows the pop from any thread; the work hops to the main thread's event loop.
pub fn show(pop: Pop) {
    DispatchQueue::main().exec_async(move || {
        if let Some(mtm) = MainThreadMarker::new() {
            present(mtm, pop);
        }
    });
}

/// Hides the pop right away, e.g. so it isn't on screen for the next selection.
pub fn dismiss(_mtm: MainThreadMarker) {
    if let Some((_, panel)) = SHOWING.take() {
        panel.close();
    }
}

thread_local! {
    /// The pop on screen, tagged so a stale close timer can't hide a newer one.
    static SHOWING: RefCell<Option<(u64, Retained<NSPanel>)>> = const { RefCell::new(None) };
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

/// One animated key path: `(seconds, value)` keyframes, starting at 0.
/// `position.*` tracks are offsets added to the layer's resting position.
type Track = (&'static str, &'static [(f64, f64)]);

const SAVED_SECS: f64 = 1.3;
const SAVED_CARD: &[Track] = &[
    // Spring in with a little overshoot, hold, then shrink to a dot.
    (
        "transform.scale",
        &[
            (0.0, 0.55),
            (0.2, 1.06),
            (0.32, 0.98),
            (0.4, 1.0),
            (1.0, 1.0),
            (1.3, 0.15),
        ],
    ),
    (
        "opacity",
        &[(0.0, 0.0), (0.12, 1.0), (1.1, 1.0), (1.3, 0.0)],
    ),
    // ...while zipping off up and to the right.
    ("position.x", &[(0.0, 0.0), (1.0, 0.0), (1.3, 56.0)]),
    ("position.y", &[(0.0, 0.0), (1.0, 0.0), (1.3, 56.0)]),
];
const SAVED_BADGE: &[Track] = &[(
    "transform.scale",
    &[(0.0, 0.0), (0.2, 0.0), (0.32, 1.25), (0.42, 1.0)],
)];
const SAVED_CHECK: &[Track] = &[("strokeEnd", &[(0.0, 0.0), (0.26, 0.0), (0.48, 1.0)])];

const FAILED_SECS: f64 = 1.2;
const FAILED_CARD: &[Track] = &[
    ("transform.scale", &[(0.0, 0.6), (0.12, 1.05), (0.2, 1.0)]),
    (
        "opacity",
        &[(0.0, 0.0), (0.1, 1.0), (0.95, 1.0), (1.2, 0.0)],
    ),
    (
        "position.x",
        &[
            (0.0, 0.0),
            (0.1, 0.0),
            (0.16, -10.0),
            (0.24, 10.0),
            (0.32, -7.0),
            (0.4, 7.0),
            (0.48, -3.0),
            (0.56, 0.0),
        ],
    ),
];

/// Reduce Motion: no scaling, travel, or shaking; just fade in and out.
const CALM_SECS: f64 = 1.3;
const CALM_CARD: &[Track] = &[(
    "opacity",
    &[(0.0, 0.0), (0.15, 1.0), (1.05, 1.0), (1.3, 0.0)],
)];

const MAX_THUMB: NSSize = NSSize::new(220.0, 150.0);
const MIN_SIDE: f64 = 36.0;
const CORNER: f64 = 10.0;
/// Room around the card for its shadow, overshoot, badge, and exit flight.
const MARGIN: f64 = 64.0;
const EDGE_INSET: f64 = 12.0;

fn present(mtm: MainThreadMarker, pop: Pop) {
    dismiss(mtm);
    let cursor = NSEvent::mouseLocation();
    let Some(screen) = screen_at(mtm, cursor) else {
        return;
    };
    let scale = screen.backingScaleFactor();
    let calm = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();

    let (card, secs) = match pop {
        Pop::Saved(path) => {
            let path = NSString::from_str(&path.to_string_lossy());
            let Some(image) = NSImage::initWithContentsOfFile(NSImage::alloc(), &path) else {
                return;
            };
            let size = thumbnail_size(image.size());
            let card = thumbnail_card(&image, size, scale);
            let (badge, check) = badge(
                26.0,
                &CGColor::new_srgb(0.2, 0.78, 0.35, 1.0),
                &[&[(0.28, 0.52), (0.44, 0.36), (0.72, 0.66)]],
                scale,
            );
            badge.setPosition(NSPoint::new(size.width - 4.0, size.height - 4.0));
            card.addSublayer(&badge);
            if calm {
                animate(&card, CALM_CARD);
                (card, CALM_SECS)
            } else {
                animate(&card, SAVED_CARD);
                animate(&badge, SAVED_BADGE);
                animate(&check, SAVED_CHECK);
                (card, SAVED_SECS)
            }
        }
        Pop::Failed => {
            let (card, _) = badge(
                44.0,
                &CGColor::new_srgb(1.0, 0.23, 0.19, 1.0),
                &[&[(0.34, 0.34), (0.66, 0.66)], &[(0.34, 0.66), (0.66, 0.34)]],
                scale,
            );
            add_shadow(&card, 22.0);
            animate(&card, if calm { CALM_CARD } else { FAILED_CARD });
            (card, if calm { CALM_SECS } else { FAILED_SECS })
        }
    };

    let card_size = card.bounds().size;
    let frame = outset(card_frame(cursor, card_size, screen.visibleFrame()), MARGIN);
    card.setPosition(NSPoint::new(
        frame.size.width / 2.0,
        frame.size.height / 2.0,
    ));
    let panel = overlay_panel(mtm, frame, &card, scale);
    panel.orderFrontRegardless();

    let id = NEXT_ID.replace(NEXT_ID.get() + 1);
    SHOWING.set(Some((id, panel)));
    let when =
        DispatchTime::try_from(Duration::from_secs_f64(secs + 0.05)).unwrap_or(DispatchTime::NOW);
    let _ = DispatchQueue::main().after(when, move || {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        if SHOWING.with_borrow(|s| s.as_ref().is_some_and(|(shown, _)| *shown == id)) {
            dismiss(mtm);
        }
    });
}

/// A transparent, click-through window above everything, on every Space, kept out of
/// screenshots so a lingering pop never lands in the next capture.
fn overlay_panel(
    mtm: MainThreadMarker,
    frame: NSRect,
    card: &CALayer,
    scale: f64,
) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    // SAFETY: we own the panel through `Retained`; AppKit must not also release it on close.
    unsafe { panel.setReleasedWhenClosed(false) };
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(false);
    panel.setIgnoresMouseEvents(true);
    panel.setHidesOnDeactivate(false);
    panel.setLevel(NSStatusWindowLevel);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
    panel.setSharingType(NSWindowSharingType::None);
    panel.setAnimationBehavior(NSWindowAnimationBehavior::None);

    // Layer-hosting view: we own the layer tree and its geometry, AppKit doesn't touch it.
    let root = CALayer::layer();
    root.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), frame.size));
    root.setContentsScale(scale);
    root.addSublayer(card);
    let view = NSView::new(mtm);
    view.setLayer(Some(&root));
    view.setWantsLayer(true);
    panel.setContentView(Some(&view));
    panel
}

/// The screenshot as a rounded, shadowed card.
fn thumbnail_card(image: &NSImage, size: NSSize, scale: f64) -> Retained<CALayer> {
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), size);
    let card = CALayer::layer();
    card.setBounds(bounds);
    add_shadow(&card, CORNER);

    let thumb = CALayer::layer();
    thumb.setFrame(bounds);
    thumb.setContentsScale(scale);
    let contents: &AnyObject = image;
    // SAFETY: on macOS, CALayer accepts an NSImage as its contents.
    unsafe { thumb.setContents(Some(contents)) };
    // SAFETY: the kCA* statics are immutable Core Animation constants.
    thumb.setContentsGravity(unsafe { kCAGravityResizeAspectFill });
    thumb.setBackgroundColor(Some(&CGColor::new_generic_gray(0.15, 1.0)));
    thumb.setCornerRadius(CORNER);
    thumb.setMasksToBounds(true);
    thumb.setBorderWidth(1.0);
    thumb.setBorderColor(Some(&CGColor::new_generic_gray(1.0, 0.35)));
    card.addSublayer(&thumb);
    card
}

fn add_shadow(layer: &CALayer, radius: f64) {
    // SAFETY: a null transform is allowed and means identity.
    let path =
        unsafe { CGPath::with_rounded_rect(layer.bounds(), radius, radius, std::ptr::null()) };
    layer.setShadowPath(Some(&path));
    layer.setShadowColor(Some(&CGColor::new_generic_gray(0.0, 1.0)));
    layer.setShadowOpacity(0.35);
    layer.setShadowRadius(14.0);
    layer.setShadowOffset(NSSize::new(0.0, -6.0));
}

/// A white-ringed colored circle with a white mark drawn from `strokes` (unit coordinates,
/// y up). Returns the circle and the mark, so the mark can be animated separately.
fn badge(
    diameter: f64,
    color: &CGColor,
    strokes: &[&[(f64, f64)]],
    scale: f64,
) -> (Retained<CALayer>, Retained<CAShapeLayer>) {
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(diameter, diameter));
    let circle = CALayer::layer();
    circle.setBounds(bounds);
    circle.setContentsScale(scale);
    circle.setCornerRadius(diameter / 2.0);
    circle.setBackgroundColor(Some(color));
    circle.setBorderWidth(2.0);
    circle.setBorderColor(Some(&CGColor::new_generic_gray(1.0, 1.0)));

    let path = CGMutablePath::new();
    for stroke in strokes {
        for (i, &(x, y)) in stroke.iter().enumerate() {
            let (x, y) = (x * diameter, y * diameter);
            // SAFETY: `path` is a live mutable path; a null transform means identity.
            unsafe {
                if i == 0 {
                    CGMutablePath::move_to_point(Some(&path), std::ptr::null(), x, y);
                } else {
                    CGMutablePath::add_line_to_point(Some(&path), std::ptr::null(), x, y);
                }
            }
        }
    }
    let mark = CAShapeLayer::layer();
    mark.setFrame(bounds);
    mark.setContentsScale(scale);
    mark.setPath(Some(&path));
    mark.setFillColor(None);
    mark.setStrokeColor(Some(&CGColor::new_generic_gray(1.0, 1.0)));
    mark.setLineWidth(diameter * 0.1);
    // SAFETY: the kCA* statics are immutable Core Animation constants.
    unsafe {
        mark.setLineCap(kCALineCapRound);
        mark.setLineJoin(kCALineJoinRound);
    }
    circle.addSublayer(&mark);
    (circle, mark)
}

/// Adds one keyframe animation per track. Each holds its last value until the pop closes.
fn animate(layer: &CALayer, tracks: &[Track]) {
    // SAFETY: the kCA* statics are immutable Core Animation constants.
    let (fill_mode, ease) = unsafe { (kCAFillModeBoth, kCAMediaTimingFunctionEaseInEaseOut) };
    for &(key_path, frames) in tracks {
        let secs = frames.last().map_or(0.0, |&(t, _)| t);
        let values: Vec<_> = frames.iter().map(|&(_, v)| NSNumber::new_f64(v)).collect();
        let values: Vec<&AnyObject> = values.iter().map(|v| v.as_ref()).collect();
        let key_times: Vec<_> = frames
            .iter()
            .map(|&(t, _)| NSNumber::new_f64(t / secs))
            .collect();
        let curves: Vec<_> = frames[1..]
            .iter()
            .map(|_| CAMediaTimingFunction::functionWithName(ease))
            .collect();

        let anim = CAKeyframeAnimation::animationWithKeyPath(Some(&NSString::from_str(key_path)));
        // SAFETY: every value is an NSNumber, which scalar key paths accept.
        unsafe { anim.setValues(Some(&NSArray::from_slice(&values))) };
        anim.setKeyTimes(Some(&NSArray::from_retained_slice(&key_times)));
        anim.setTimingFunctions(Some(&NSArray::from_retained_slice(&curves)));
        anim.setAdditive(key_path.starts_with("position"));
        anim.setDuration(secs);
        anim.setFillMode(fill_mode);
        anim.setRemovedOnCompletion(false);
        layer.addAnimation_forKey(&anim, Some(&NSString::from_str(key_path)));
    }
}

fn screen_at(mtm: MainThreadMarker, p: NSPoint) -> Option<Retained<NSScreen>> {
    NSScreen::screens(mtm)
        .iter()
        .find(|s| {
            let f = s.frame();
            (f.origin.x..=f.origin.x + f.size.width).contains(&p.x)
                && (f.origin.y..=f.origin.y + f.size.height).contains(&p.y)
        })
        .or_else(|| NSScreen::mainScreen(mtm))
}

/// Fits the image in `MAX_THUMB` without upscaling, keeping tiny captures visible.
fn thumbnail_size(image: NSSize) -> NSSize {
    let fit = (MAX_THUMB.width / image.width)
        .min(MAX_THUMB.height / image.height)
        .min(1.0);
    NSSize::new(
        (image.width * fit).max(MIN_SIDE),
        (image.height * fit).max(MIN_SIDE),
    )
}

/// Centers the card on the cursor, nudged to stay fully inside the screen's visible area.
fn card_frame(cursor: NSPoint, card: NSSize, visible: NSRect) -> NSRect {
    let clamp = |v: f64, lo: f64, hi: f64| v.min(hi).max(lo);
    let x = clamp(
        cursor.x - card.width / 2.0,
        visible.origin.x + EDGE_INSET,
        visible.origin.x + visible.size.width - EDGE_INSET - card.width,
    );
    let y = clamp(
        cursor.y - card.height / 2.0,
        visible.origin.y + EDGE_INSET,
        visible.origin.y + visible.size.height - EDGE_INSET - card.height,
    );
    NSRect::new(NSPoint::new(x, y), card)
}

fn outset(r: NSRect, by: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(r.origin.x - by, r.origin.y - by),
        NSSize::new(r.size.width + 2.0 * by, r.size.height + 2.0 * by),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: NSRect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0));

    #[test]
    fn thumbnail_fits_box_and_keeps_aspect() {
        let s = thumbnail_size(NSSize::new(1100.0, 500.0));
        assert_eq!((s.width, s.height), (220.0, 100.0));
        let s = thumbnail_size(NSSize::new(300.0, 900.0));
        assert_eq!((s.width, s.height), (50.0, 150.0));
    }

    #[test]
    fn thumbnail_never_upscales_but_stays_visible() {
        let s = thumbnail_size(NSSize::new(80.0, 60.0));
        assert_eq!((s.width, s.height), (80.0, 60.0));
        let s = thumbnail_size(NSSize::new(4000.0, 10.0));
        assert_eq!((s.width, s.height), (220.0, MIN_SIDE));
    }

    #[test]
    fn card_centers_on_cursor() {
        let r = card_frame(
            NSPoint::new(700.0, 400.0),
            NSSize::new(200.0, 100.0),
            SCREEN,
        );
        assert_eq!((r.origin.x, r.origin.y), (600.0, 350.0));
    }

    #[test]
    fn card_stays_on_screen_near_edges() {
        let card = NSSize::new(200.0, 100.0);
        let r = card_frame(NSPoint::new(1439.0, 899.0), card, SCREEN);
        assert_eq!(
            (r.origin.x, r.origin.y),
            (1440.0 - 12.0 - 200.0, 900.0 - 12.0 - 100.0)
        );
        let r = card_frame(NSPoint::new(0.0, 0.0), card, SCREEN);
        assert_eq!((r.origin.x, r.origin.y), (12.0, 12.0));
    }

    /// Builds the real layer tree and animations without a window, so every Core Animation
    /// call runs (and objc2 checks its message encodings in debug builds).
    #[test]
    fn builds_layers_and_animations_off_screen() {
        let size = NSSize::new(120.0, 80.0);
        let card = thumbnail_card(&NSImage::new(), size, 2.0);
        let green = CGColor::new_srgb(0.2, 0.78, 0.35, 1.0);
        let (badge, check) = badge(
            26.0,
            &green,
            &[&[(0.28, 0.52), (0.44, 0.36), (0.72, 0.66)]],
            2.0,
        );
        card.addSublayer(&badge);
        animate(&card, SAVED_CARD);
        animate(&badge, SAVED_BADGE);
        animate(&check, SAVED_CHECK);

        let keys = |layer: &CALayer| layer.animationKeys().map_or(0, |k| k.count());
        assert_eq!(keys(&card), SAVED_CARD.len());
        assert_eq!(keys(&badge), SAVED_BADGE.len());
        assert_eq!(keys(&check), SAVED_CHECK.len());
        assert!(check.path().is_some());
    }

    /// Core Animation silently misplays keyframes that don't start at 0 or go backwards,
    /// and the pop must not close before its slowest track ends.
    #[test]
    fn timelines_are_well_formed() {
        let timelines = [
            (SAVED_CARD, SAVED_SECS),
            (SAVED_BADGE, SAVED_SECS),
            (SAVED_CHECK, SAVED_SECS),
            (FAILED_CARD, FAILED_SECS),
            (CALM_CARD, CALM_SECS),
        ];
        for (tracks, secs) in timelines {
            for &(key_path, frames) in tracks {
                assert_eq!(frames[0].0, 0.0, "{key_path} must start at 0");
                assert!(
                    frames.windows(2).all(|w| w[0].0 < w[1].0),
                    "{key_path} goes backwards"
                );
                assert!(
                    frames.last().unwrap().0 <= secs,
                    "{key_path} outlives the pop"
                );
            }
        }
    }
}
