pub mod capture;
pub mod ocr;

use capture::{CaptureState, CropSelection, MonitorInfo};
use std::sync::Arc;
use tauri::{
    http::{header::CONTENT_TYPE, Response, StatusCode},
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[tauri::command]
fn get_monitors(state: State<'_, Arc<CaptureState>>) -> Result<Vec<MonitorInfo>, String> {
    state.freeze_all_monitors()
}

#[tauri::command]
fn confirm_crop(
    app: AppHandle,
    state: State<'_, Arc<CaptureState>>,
    monitor_id: u32,
    logical_x: f64,
    logical_y: f64,
    logical_w: f64,
    logical_h: f64,
    physical_x: u32,
    physical_y: u32,
    physical_w: u32,
    physical_h: u32,
) -> Result<String, String> {
    let crop_id = uuid::Uuid::new_v4().to_string();
    let crop = CropSelection {
        monitor_id,
        physical_x,
        physical_y,
        physical_w,
        physical_h,
    };

    state.crop_and_store(&crop_id, crop)?;

    // Cerrar ventanas de overlay
    close_all_overlays(&app);

    // Abrir la ventana PinWindow para este recorte
    spawn_pin_window(&app, &crop_id, logical_x, logical_y, logical_w, logical_h)?;

    Ok(crop_id)
}

#[tauri::command]
fn cancel_capture(app: AppHandle) -> Result<(), String> {
    close_all_overlays(&app);
    Ok(())
}

#[tauri::command]
fn trigger_capture(app: AppHandle) -> Result<(), String> {
    start_capture_flow(&app)
}

#[tauri::command]
fn toggle_ghost_mode(app: AppHandle, pin_id: String, enabled: bool) -> Result<(), String> {
    let label = format!("pin-{}", pin_id);
    if let Some(win) = app.get_webview_window(&label) {
        win.set_ignore_cursor_events(enabled)
            .map_err(|e| format!("Fallo al configurar eventos de cursor: {e}"))?;
    }
    Ok(())
}

fn close_all_overlays(app: &AppHandle) {
    let windows = app.webview_windows();
    for (label, win) in windows {
        if label.starts_with("overlay-") {
            let _ = win.close();
        }
    }
}

pub fn spawn_pin_window(
    app: &AppHandle,
    pin_id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    let label = format!("pin-{}", pin_id);
    let url = format!("/pin/{}", pin_id);

    tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
        .title("Screen-Pin")
        .inner_size(w, h)
        .position(x, y)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .resizable(true)
        .shadow(true)
        .build()
        .map_err(|e| format!("Fallo al instanciar ventana PinWindow: {e}"))?;

    Ok(())
}

pub fn start_capture_flow(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<CaptureState>>();
    let monitors = state.freeze_all_monitors()?;

    close_all_overlays(app);

    for mon in &monitors {
        let label = format!("overlay-{}", mon.id);
        let url = format!("/overlay?monitor_id={}", mon.id);

        let logical_x = mon.x as f64;
        let logical_y = mon.y as f64;
        let logical_w = mon.width as f64 / mon.scale_factor as f64;
        let logical_h = mon.height as f64 / mon.scale_factor as f64;

        let win = tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
            .title("Screen-Pin Overlay")
            .inner_size(logical_w, logical_h)
            .position(logical_x, logical_y)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(true)
            .resizable(false)
            .build()
            .map_err(|e| format!("Fallo al abrir overlay de captura: {e}"))?;

        let _ = win.set_focus();
    }

    Ok(())
}

pub fn run() {
    let capture_state = Arc::new(CaptureState::new());
    let state_for_protocol = Arc::clone(&capture_state);

    tauri::Builder::default()
        .manage(capture_state)
        .register_uri_scheme_protocol("pinasset", move |_app, request| {
            let path = request.uri().path();
            // path format: /<crop_id> o /monitor/<id>
            let trimmed = path.trim_start_matches('/');

            if let Some(mon_str) = trimmed.strip_prefix("monitor/") {
                if let Ok(mon_id) = mon_str.parse::<u32>() {
                    let frames = state_for_protocol.monitor_frames.lock().unwrap();
                    if let Some(frame) = frames.get(&mon_id) {
                        let mut png_bytes = Vec::new();
                        let mut cursor = std::io::Cursor::new(&mut png_bytes);
                        if frame
                            .write_to(&mut cursor, image::ImageFormat::Png)
                            .is_ok()
                        {
                            return Response::builder()
                                .status(StatusCode::OK)
                                .header(CONTENT_TYPE, "image/png")
                                .body(png_bytes)
                                .unwrap();
                        }
                    }
                }
            } else if let Some(bytes) = state_for_protocol.get_crop_bytes(trimmed) {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "image/png")
                    .body(bytes)
                    .unwrap();
            }

            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new())
                .unwrap()
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Bandeja del sistema (Tray Icon)
            let quit_i = MenuItem::with_id(app, "quit", "Salir de Screen-Pin", true, None::<&str>)?;
            let capture_i = MenuItem::with_id(
                app,
                "capture",
                "Capturar Recorte (Alt+Shift+S)",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&capture_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Screen-Pin - Recortes flotantes con memoria")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "capture" => {
                        let _ = start_capture_flow(app);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let _ = start_capture_flow(app);
                    }
                })
                .build(app)?;

            // Atajos Globales: Alt+Shift+S (Captura)
            let capture_shortcut: Shortcut = "alt+shift+s".parse().map_err(|e| {
                tauri::Error::PluginInitialization("shortcut".to_string(), format!("Invalid shortcut: {e:?}"))
            })?;

            let app_handle = app.handle().clone();
            app.global_shortcut().on_shortcut(capture_shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = start_capture_flow(&app_handle);
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_monitors,
            confirm_crop,
            cancel_capture,
            trigger_capture,
            toggle_ghost_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
