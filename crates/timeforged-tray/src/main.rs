use std::sync::{Arc, Mutex};
use std::time::Duration;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

use timeforged_core::config::CliConfig;

mod poller;

fn main() {
    let config = CliConfig::load();
    let server_url = config.server_url.clone();
    let api_key = config.api_key.clone().unwrap_or_default();
    let dashboard_url = format!("{}/", server_url.trim_end_matches('/'));

    // Build menu
    let menu = Menu::new();
    let item_open = MenuItem::new("Open Dashboard", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    let _ = menu.append(&item_open);
    let _ = menu.append(&item_quit);

    let open_id = item_open.id().clone();
    let quit_id = item_quit.id().clone();

    // Load icon
    let icon = load_icon();

    // Create tray icon
    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("TimeForged: connecting...")
        .with_menu(Box::new(menu))
        .with_title("TimeForged")
        .build()
        .expect("failed to create tray icon");

    // Shared tooltip state
    let tooltip_state: Arc<Mutex<String>> = Arc::new(Mutex::new("TimeForged: connecting...".into()));

    // Spawn poller in background thread with its own tokio runtime
    let poller_state = tooltip_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        rt.block_on(poller::run(server_url, api_key, poller_state));
    });

    // Platform-specific event loop
    run_event_loop(tray, tooltip_state, dashboard_url, open_id, quit_id);
}

#[cfg(target_os = "linux")]
fn run_event_loop(
    tray: tray_icon::TrayIcon,
    tooltip_state: Arc<Mutex<String>>,
    dashboard_url: String,
    open_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
) {
    // On Linux, tray-icon uses GTK under the hood via libappindicator.
    // We need a simple polling loop since there's no GTK main loop.
    let menu_rx = MenuEvent::receiver();
    let tray_rx = TrayIconEvent::receiver();

    loop {
        // Process menu events
        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == &open_id {
                let _ = open::that(&dashboard_url);
            } else if event.id() == &quit_id {
                drop(tray);
                std::process::exit(0);
            }
        }

        // Process tray click events
        if let Ok(TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. }) = tray_rx.try_recv() {
            let _ = open::that(&dashboard_url);
        }

        // Update tooltip
        if let Ok(tooltip) = tooltip_state.lock() {
            let _ = tray.set_tooltip(Some(tooltip.as_str()));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
fn run_event_loop(
    tray: tray_icon::TrayIcon,
    tooltip_state: Arc<Mutex<String>>,
    dashboard_url: String,
    open_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    let menu_rx = MenuEvent::receiver();
    let tray_rx = TrayIconEvent::receiver();

    // Spawn tooltip updater — updates every 5s within the Win32 message cadence
    let tooltip_tray = &tray;
    let tooltip_clone = tooltip_state.clone();

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));
        // Can't update tray from another thread on Windows;
        // tooltip gets updated in main loop below
        let _ = &tooltip_clone;
    });

    // Win32 message loop
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    let mut last_tooltip_update = std::time::Instant::now();

    loop {
        // Non-blocking peek for Win32 messages
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Process menu events
        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == &open_id {
                let _ = open::that(&dashboard_url);
            } else if event.id() == &quit_id {
                drop(tray);
                std::process::exit(0);
            }
        }

        // Process tray click events
        if let Ok(TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. }) = tray_rx.try_recv() {
            let _ = open::that(&dashboard_url);
        }

        // Update tooltip every 5s
        if last_tooltip_update.elapsed() >= Duration::from_secs(5) {
            if let Ok(tooltip) = tooltip_state.lock() {
                let _ = tray.set_tooltip(Some(tooltip.as_str()));
            }
            last_tooltip_update = std::time::Instant::now();
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn run_event_loop(
    tray: tray_icon::TrayIcon,
    tooltip_state: Arc<Mutex<String>>,
    dashboard_url: String,
    open_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
) {
    // macOS and others: simple polling loop
    let menu_rx = MenuEvent::receiver();
    loop {
        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == &open_id {
                let _ = open::that(&dashboard_url);
            } else if event.id() == &quit_id {
                drop(tray);
                std::process::exit(0);
            }
        }
        if let Ok(tooltip) = tooltip_state.lock() {
            let _ = tray.set_tooltip(Some(tooltip.as_str()));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn load_icon() -> Icon {
    let icon_bytes = include_bytes!("../../timeforged/web/public/favicon-32.png");

    let img = image::load_from_memory_with_format(icon_bytes, image::ImageFormat::Png)
        .expect("failed to decode embedded icon");

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), w, h).expect("failed to create icon")
}
