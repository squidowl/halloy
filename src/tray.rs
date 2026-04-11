use iced::Subscription;

use crate::Message;

pub fn init() -> bool {
    #[cfg(target_os = "linux")]
    return linux::init();

    #[cfg(not(target_os = "linux"))]
    return desktop::init();
}

pub fn subscription() -> Subscription<Message> {
    #[cfg(target_os = "linux")]
    return linux::subscription();

    #[cfg(not(target_os = "linux"))]
    return desktop::subscription();
}

// Linux uses ksni which speaks the StatusNotifierItem DBus protocol directly.
// This gives us separate left-click (activate) and right-click (menu) events.
// ksni needs its own single-threaded tokio runtime running in a background thread.
#[cfg(target_os = "linux")]
mod linux {
    use std::sync::{Mutex, OnceLock};

    use iced::Subscription;
    use ksni::TrayMethods as _;

    use crate::Message;

    #[derive(Debug, Clone)]
    enum TrayEvent {
        Toggle,
        Show,
        Quit,
    }

    static TRAY_TX: OnceLock<tokio::sync::mpsc::Sender<TrayEvent>> =
        OnceLock::new();
    static TRAY_RX: OnceLock<
        Mutex<Option<tokio::sync::mpsc::Receiver<TrayEvent>>>,
    > = OnceLock::new();

    struct HalloyTray;

    impl ksni::Tray for HalloyTray {
        fn id(&self) -> String {
            "halloy".to_string()
        }

        fn title(&self) -> String {
            "Halloy".to_string()
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            let bytes = include_bytes!(
                "../assets/linux/icons/hicolor/32x32/apps/org.squidowl.halloy.png"
            );
            if let Ok(img) = image::load_from_memory(bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                // SNI expects ARGB rather than RGBA
                let argb: Vec<u8> = rgba
                    .chunks(4)
                    .flat_map(|px| [px[3], px[0], px[1], px[2]])
                    .collect();
                vec![ksni::Icon {
                    width: w as i32,
                    height: h as i32,
                    data: argb,
                }]
            } else {
                vec![]
            }
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            send(TrayEvent::Toggle);
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            vec![
                ksni::MenuItem::Standard(ksni::menu::StandardItem {
                    label: "Show".into(),
                    activate: Box::new(|_| send(TrayEvent::Show)),
                    ..Default::default()
                }),
                ksni::MenuItem::Separator,
                ksni::MenuItem::Standard(ksni::menu::StandardItem {
                    label: "Quit".into(),
                    activate: Box::new(|_| send(TrayEvent::Quit)),
                    ..Default::default()
                }),
            ]
        }
    }

    fn send(event: TrayEvent) {
        if let Some(tx) = TRAY_TX.get() {
            let _ = tx.try_send(event);
        }
    }

    pub fn init() -> bool {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let _ = TRAY_TX.set(tx);
        let _ = TRAY_RX.set(Mutex::new(Some(rx)));

        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                match HalloyTray.spawn().await {
                    Ok(_handle) => futures::future::pending::<()>().await,
                    Err(e) => log::warn!("System tray unavailable: {e}"),
                }
            });
        });

        true
    }

    pub fn subscription() -> Subscription<Message> {
        Subscription::run(|| {
            iced::stream::channel(
                16,
                |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use futures::SinkExt as _;

                    let mut rx = TRAY_RX
                        .get()
                        .and_then(|m| m.lock().ok()?.take())
                        .expect("tray receiver available");

                    while let Some(event) = rx.recv().await {
                        let msg = match event {
                            TrayEvent::Toggle => Message::TrayIconClicked,
                            TrayEvent::Show => Message::TrayMenuShow,
                            TrayEvent::Quit => Message::TrayMenuQuit,
                        };
                        let _ = output.send(msg).await;
                    }
                },
            )
        })
    }
}

// Windows and macOS use the tray-icon crate which provides native implementations
// on each platform. The TrayIcon is kept in thread-local storage since AppKit
// objects on macOS must stay on the main thread.
#[cfg(not(target_os = "linux"))]
mod desktop {
    use std::cell::RefCell;
    use std::time::Duration;

    use iced::Subscription;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

    use crate::Message;

    thread_local! {
        static ICON: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
    }

    pub fn init() -> bool {
        let icon = match load_icon() {
            Some(i) => i,
            None => {
                log::warn!("Failed to load tray icon image.");
                return false;
            }
        };

        let menu = Menu::new();
        let show = MenuItem::with_id("show", "Show", true, None);
        let quit = MenuItem::with_id("quit", "Quit", true, None);

        if menu
            .append_items(&[&show, &PredefinedMenuItem::separator(), &quit])
            .is_err()
        {
            return false;
        }

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("Halloy")
            .build()
        {
            Ok(tray) => {
                ICON.with(|cell| *cell.borrow_mut() = Some(tray));
                true
            }
            Err(e) => {
                log::warn!("System tray unavailable: {e}");
                false
            }
        }
    }

    pub fn subscription() -> Subscription<Message> {
        Subscription::run(|| {
            iced::stream::channel(
                16,
                |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use futures::SinkExt as _;
                    loop {
                        while let Ok(event) =
                            TrayIconEvent::receiver().try_recv()
                        {
                            if let TrayIconEvent::Click {
                                button: tray_icon::MouseButton::Left,
                                button_state: tray_icon::MouseButtonState::Up,
                                ..
                            } = event
                            {
                                let _ =
                                    output.send(Message::TrayIconClicked).await;
                            }
                        }
                        while let Ok(event) = MenuEvent::receiver().try_recv() {
                            let msg = match event.id.0.as_str() {
                                "show" => Some(Message::TrayMenuShow),
                                "quit" => Some(Message::TrayMenuQuit),
                                _ => None,
                            };
                            if let Some(msg) = msg {
                                let _ = output.send(msg).await;
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                },
            )
        })
    }

    fn load_icon() -> Option<tray_icon::Icon> {
        let bytes = include_bytes!(
            "../assets/linux/icons/hicolor/32x32/apps/org.squidowl.halloy.png"
        );
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).ok()
    }
}
