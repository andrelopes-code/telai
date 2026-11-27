use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::app::show_main_window;
use crate::live;
use crate::text::text;

const TRAY_ID: &str = "main";
const OPEN: &str = "open";
const STOP: &str = "stop";
const QUIT: &str = "quit";

/// Menu entry whose state follows the stream.
struct StopItem(MenuItem<Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let text = text();
    let open = MenuItem::with_id(app, OPEN, text.open, true, None::<&str>)?;
    let stop = MenuItem::with_id(app, STOP, text.stop, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, text.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &stop, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(text.idle_tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN => show_main_window(app),
            STOP => live::stop(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(open_on_left_click);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    app.manage(StopItem(stop));

    Ok(())
}

pub fn set_live(app: &AppHandle, live: bool) {
    let text = text();
    let tooltip = if live {
        text.live_tooltip
    } else {
        text.idle_tooltip
    };

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
    if let Some(stop) = app.try_state::<StopItem>() {
        let _ = stop.0.set_enabled(live);
    }
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "the tray callback owns the event"
)]
fn open_on_left_click(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}
