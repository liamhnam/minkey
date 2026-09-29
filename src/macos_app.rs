// NSApplication helpers: Minkey lives in the menu bar (no Dock icon), like OpenKey's LSUIElement

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

/// Hides the Dock icon. Must run on the main thread after the event loop has started
/// (winit resets the activation policy while launching).
pub fn set_accessory_app() {
    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }
}

/// Brings Minkey's windows to the front; an accessory app is not activated automatically.
#[allow(deprecated)]
pub fn activate() {
    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
}
