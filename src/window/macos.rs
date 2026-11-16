use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::Once;

use block2::RcBlock;
use iced::{Subscription, event, mouse};
use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{
    NSApplication, NSEvent, NSEventMask, NSMenu, NSMenuItem, NSWindow,
    NSWindowButton, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSString, NSUserDefaults, ns_string};

thread_local! {
    // set on a titlebar double click, read in handle_click. iced may still
    // have eaten the click, so that's checked there
    static PENDING_WINDOW: RefCell<Option<Retained<NSWindow>>> =
        const { RefCell::new(None) };
}

pub fn setup() {
    static SETUP: Once = Once::new();

    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("macOS window setup skipped, not on the main thread");
        return;
    };

    SETUP.call_once(|| {
        add_window_menu(mtm);
        watch_titlebar_clicks(mtm);
    });
}

fn add_window_menu(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);

    let Some(menu_bar) = app.mainMenu() else {
        return;
    };

    let window_menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!("Window"));

    // macOS adds the tiling items to this menu by itself
    for (title, action, key) in [
        ("Minimize", sel!(performMiniaturize:), "m"),
        ("Zoom", sel!(performZoom:), ""),
    ] {
        // SAFETY: standard selectors, the responder chain picks them up
        let entry = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        window_menu.addItem(&entry);
    }

    let window_item = NSMenuItem::new(mtm);
    window_item.setSubmenu(Some(&window_menu));
    menu_bar.addItem(&window_item);

    app.setWindowsMenu(Some(&window_menu));
}

fn watch_titlebar_clicks(mtm: MainThreadMarker) {
    let monitor_block = RcBlock::new(move |event: NonNull<NSEvent>| {
        PENDING_WINDOW.with_borrow_mut(|pending| *pending = None);

        // SAFETY: the event only lives for this callback
        let ns_event = unsafe { event.as_ref() };

        if ns_event.clickCount() == 2
            && let Some(window) = ns_event.window(mtm)
            && is_titlebar_click(&window, ns_event)
        {
            PENDING_WINDOW.with_borrow_mut(|pending| *pending = Some(window));
        }

        // don't swallow it, iced needs the click too
        event.as_ptr()
    });

    // SAFETY: the block hands back the same event
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::LeftMouseDown,
            &monitor_block,
        )
    };

    // never removed, so leak it on purpose
    std::mem::forget(monitor);
}

fn is_titlebar_click(window: &NSWindow, event: &NSEvent) -> bool {
    let style = window.styleMask();

    // only our transparent titlebar windows. fullscreen has no titlebar
    if !style.contains(NSWindowStyleMask::FullSizeContentView)
        || style.contains(NSWindowStyleMask::FullScreen)
    {
        return false;
    }

    let click = event.locationInWindow();
    let content = window.contentLayoutRect();

    if click.y <= content.origin.y + content.size.height {
        return false;
    }

    // traffic lights get their own clicks, don't steal them
    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = window.standardWindowButton(kind) {
            let local = button.convertPoint_fromView(click, None);

            if button.mouse_inRect(local, button.bounds()) {
                return false;
            }
        }
    }

    true
}

/// Left presses. The bool says whether iced ignored the press.
pub fn clicks() -> Subscription<bool> {
    event::listen_with(|event, status, _window| match event {
        iced::Event::Mouse(mouse::Event::ButtonPressed {
            button: mouse::Button::Left,
            ..
        }) => Some(status == event::Status::Ignored),
        _ => None,
    })
}

/// Zooms or minimizes if the last press was a titlebar double click
/// and iced didn't handle it.
pub fn handle_click(ignored: bool) {
    let window = PENDING_WINDOW.with_borrow_mut(Option::take);

    let Some(window) = window else {
        return;
    };

    if !ignored {
        return;
    }

    let action = NSUserDefaults::standardUserDefaults()
        .stringForKey(ns_string!("AppleActionOnDoubleClick"))
        .map(|action| action.to_string());

    match action.as_deref() {
        // Sequoia's Fill has no public API, zoom is the closest
        None | Some("Maximize" | "Fill") => window.performZoom(None),
        Some("Minimize") => window.performMiniaturize(None),
        // "None" or an unknown value, leave the window alone
        _ => {}
    }
}
