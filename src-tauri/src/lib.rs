pub mod capture;
pub mod memory_db;
pub mod ocr;

use capture::{CaptureState, CropSelection, MonitorInfo};
use memory_db::{MemoryDatabase, PinRecord};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Emitter, Manager, State,
    http::{Response, StatusCode, header::CONTENT_TYPE},
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct ShortcutWarnings(Vec<String>);

#[tauri::command]
fn get_shortcut_warnings(state: State<'_, ShortcutWarnings>) -> Vec<String> {
    state.0.clone()
}

#[tauri::command]
fn get_monitors(state: State<'_, Arc<CaptureState>>) -> Result<Vec<MonitorInfo>, String> {
    state.get_monitors()
}

#[tauri::command]
async fn confirm_crop(
    app: AppHandle,
    state: State<'_, Arc<CaptureState>>,
    db: State<'_, Arc<MemoryDatabase>>,
    selection: CropSelection,
) -> Result<String, String> {
    let CropSelection {
        monitor_id,
        physical_x,
        physical_y,
        physical_w,
        physical_h,
    } = selection;
    if physical_w < 4 || physical_h < 4 {
        return Err("La selección debe medir al menos 4 × 4 píxeles".into());
    }
    let monitor = state
        .get_monitors()?
        .into_iter()
        .find(|m| m.id == monitor_id)
        .ok_or("El monitor ya no está disponible")?;
    let crop_id = uuid::Uuid::new_v4().to_string();
    let crop = CropSelection {
        monitor_id,
        physical_x,
        physical_y,
        physical_w,
        physical_h,
    };

    let (png_bytes, sub_img) = state.crop_and_store(&crop_id, crop)?;
    let logical_x = f64::from(monitor.x) + f64::from(physical_x);
    let logical_y = f64::from(monitor.y) + f64::from(physical_y);
    let logical_w = f64::from(sub_img.width()) / f64::from(monitor.scale_factor);
    let logical_h = f64::from(sub_img.height()) / f64::from(monitor.scale_factor);

    // Guardar imagen en disco local en formato WebP / PNG
    let img_filename = format!("{}.png", crop_id);
    let img_path = db.data_dir.join(&img_filename);
    std::fs::write(&img_path, &png_bytes).map_err(|e| {
        state.release_crop(&crop_id);
        format!("No se pudo guardar el recorte: {e}")
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Guardar registro inicial en SQLite Memoria
    let initial_pin = PinRecord {
        id: crop_id.clone(),
        created_at: now,
        archived_at: None,
        image_path: img_filename,
        pos_x: logical_x,
        pos_y: logical_y,
        width: logical_w,
        height: logical_h,
        monitor_id,
        raw_text: String::new(),
        ocr_json: String::new(),
        entity_type: None,
        is_sensitive: false,
        expires_at: None,
        is_pinned_forever: false,
        ocr_status: "pending".into(),
    };
    if let Err(error) = db.save_pin(&initial_pin) {
        state.release_crop(&crop_id);
        let _ = std::fs::remove_file(&img_path);
        return Err(error);
    }

    // Cerrar ventanas de overlay
    close_all_overlays(&app);
    state.clear_frames();
    let _ = app.emit("memory:changed", ());

    // Abrir la ventana PinWindow para este recorte de forma instantánea
    spawn_pin_window(&app, &crop_id, logical_x, logical_y, logical_w, logical_h)?;

    // Procesamiento OCR asíncrono en segundo plano sin bloquear el hilo de la UI
    let app_for_ocr = app.clone();
    let pin_id_for_ocr = crop_id.clone();
    let db_for_ocr = Arc::clone(&db);
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            match crate::ocr::windows_ocr::run_windows_ocr(&pin_id_for_ocr, &sub_img) {
                Ok(payload) => {
                    if let Err(error) = db_for_ocr.update_ocr(&pin_id_for_ocr, Some(&payload)) {
                        eprintln!("No se pudo guardar el texto: {error}");
                    }

                    let event_name = format!("ocr:ready:{}", pin_id_for_ocr);
                    let _ = app_for_ocr.emit_to(format!("pin-{pin_id_for_ocr}"), &event_name, &payload);
                    let _ = app_for_ocr.emit_to(format!("pin-toolbar-{pin_id_for_ocr}"), &event_name, payload);
                }
                Err(err) => {
                    eprintln!("Aviso: fallo en OCR para {pin_id_for_ocr}: {err}");
                    let _ = db_for_ocr.update_ocr(&pin_id_for_ocr, None);
                    let event_name = format!("ocr:error:{}", pin_id_for_ocr);
                    let _ = app_for_ocr.emit_to(format!("pin-{pin_id_for_ocr}"), &event_name, ());
                    let _ = app_for_ocr.emit_to(format!("pin-toolbar-{pin_id_for_ocr}"), &event_name, ());
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = db_for_ocr.update_ocr(&pin_id_for_ocr, None);
        let _ = app_for_ocr.emit("memory:changed", ());
    });

    Ok(crop_id)
}

#[tauri::command]
async fn cancel_capture(app: AppHandle) -> Result<(), String> {
    close_all_overlays(&app);
    app.state::<Arc<CaptureState>>().clear_frames();
    Ok(())
}

#[tauri::command]
async fn trigger_capture(app: AppHandle) -> Result<(), String> {
    start_capture_flow(&app)
}

#[tauri::command]
async fn toggle_ghost_mode(app: AppHandle, pin_id: String, enabled: bool) -> Result<(), String> {
    let label = format!("pin-{}", pin_id);
    if let Some(win) = app.get_webview_window(&label) {
        win.set_ignore_cursor_events(enabled)
            .map_err(|e| format!("Fallo al configurar eventos de cursor: {e}"))?;
        app.emit_to(&label, "ghost:changed", enabled)
            .map_err(|e| e.to_string())?;
        let _ = app.emit_to(format!("pin-toolbar-{pin_id}"), "ghost:changed", enabled);
    } else {
        return Err("El recorte ya no está abierto".into());
    }
    Ok(())
}

fn restore_interaction(app: &AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with("pin-toolbar-") {
            let _ = app.emit_to(&label, "ghost:changed", false);
        } else if label.starts_with("pin-") {
            let _ = win.set_ignore_cursor_events(false);
            let _ = app.emit_to(&label, "ghost:changed", false);
        }
    }
}

#[tauri::command]
fn get_pin(db: State<'_, Arc<MemoryDatabase>>, pin_id: String) -> Result<PinSnapshot, String> {
    let pin = db
        .get_pin(&pin_id)?
        .ok_or("El recorte ya no está disponible")?;
    let (entities, _) = ocr::extract_smart_entities(&pin.raw_text);
    Ok(PinSnapshot { pin, entities })
}

#[derive(serde::Serialize)]
struct PinSnapshot {
    pin: PinRecord,
    entities: Vec<ocr::SmartEntity>,
}

#[tauri::command]
fn get_memory_pins(db: State<'_, Arc<MemoryDatabase>>) -> Result<Vec<PinRecord>, String> {
    db.get_recent_pins(50)
}

#[tauri::command]
fn search_memory_pins(
    db: State<'_, Arc<MemoryDatabase>>,
    query: String,
) -> Result<Vec<PinRecord>, String> {
    db.search_pins(&query)
}

#[tauri::command]
async fn archive_pin_to_memory(
    app: AppHandle,
    state: State<'_, Arc<CaptureState>>,
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<(), String> {
    db.archive_pin(&pin_id)?;
    if let Some(win) = app.get_webview_window(&format!("pin-{pin_id}")) {
        win.close().map_err(|e| e.to_string())?;
    }
    state.release_crop(&pin_id);
    let _ = app.emit("memory:changed", ());
    Ok(())
}

#[tauri::command]
fn toggle_retention(
    app: AppHandle,
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<bool, String> {
    let retained = db.toggle_pinned_forever(&pin_id)?;
    let _ = app.emit("memory:changed", ());
    Ok(retained)
}

#[tauri::command]
async fn repin_from_memory(
    app: AppHandle,
    state: State<'_, Arc<CaptureState>>,
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<(), String> {
    let pin = db
        .get_pin(&pin_id)?
        .ok_or_else(|| "Pin no encontrado en base de datos".to_string())?;

    // Asegurar que la imagen esté en el buffer de memoria pinasset://
    if state.get_crop_bytes(&pin.id).is_none() {
        let full_path = db.data_dir.join(&pin.image_path);
        let bytes =
            std::fs::read(&full_path).map_err(|e| format!("No se puede abrir la imagen: {e}"))?;
        state
            .pinned_crops
            .lock()
            .map_err(|e| e.to_string())?
            .insert(pin.id.clone(), bytes);
    }

    spawn_pin_window(&app, &pin.id, pin.pos_x, pin.pos_y, pin.width, pin.height)?;

    Ok(())
}

#[tauri::command]
async fn open_memory_drawer(app: AppHandle) -> Result<(), String> {
    show_memory_drawer(&app)
}

fn show_memory_drawer(app: &AppHandle) -> Result<(), String> {
    restore_interaction(app);
    let label = "memory-drawer";
    if let Some(win) = app.get_webview_window(label) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        let _ = win.emit("memory:changed", ());
        return Ok(());
    }

    let url = "/?view=memory";
    tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
        .title("Memoria - Screen-Pin")
        .inner_size(420.0, 700.0)
        .min_inner_size(320.0, 400.0)
        .decorations(true)
        .always_on_top(true)
        .resizable(true)
        .build()
        .map_err(|e| format!("Fallo al abrir ventana de Memoria: {e}"))?;

    Ok(())
}

// WebView2 window creation must run outside synchronous IPC and native event handlers.
fn run_window_action(app: &AppHandle, action: fn(&AppHandle) -> Result<(), String>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = action(&app) {
            let _ = app.emit("app:error", error);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }
    });
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
    if let Some(win) = app.get_webview_window(&label) {
        win.set_ignore_cursor_events(false)
            .map_err(|e| e.to_string())?;
        app.emit_to(&label, "ghost:changed", false)
            .map_err(|e| e.to_string())?;
        let toolbar = show_pin_toolbar(app, &win, pin_id)?;
        let _ = app.emit_to(toolbar.label(), "ghost:changed", false);
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let url = format!("/?view=pin&pin_id={}", pin_id);

    let win = tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
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
    win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
        .map_err(|e| e.to_string())?;
    win.set_size(tauri::LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    let toolbar = show_pin_toolbar(app, &win, pin_id)?;
    let closed_app = app.clone();
    let closed_id = pin_id.to_owned();
    let follow_pin = win.clone();
    win.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                let _ = position_pin_toolbar(&follow_pin, &toolbar);
            }
            tauri::WindowEvent::Destroyed => {
                let _ = toolbar.close();
                closed_app.state::<Arc<CaptureState>>().release_crop(&closed_id);
                let _ = closed_app.state::<Arc<MemoryDatabase>>().archive_pin(&closed_id);
                let _ = closed_app.emit("memory:changed", ());
            }
            _ => {}
        }
    });
    let _ = win.set_focus();

    Ok(())
}

fn show_pin_toolbar(
    app: &AppHandle,
    pin: &tauri::WebviewWindow,
    pin_id: &str,
) -> Result<tauri::WebviewWindow, String> {
    let label = format!("pin-toolbar-{pin_id}");
    let toolbar = if let Some(existing) = app.get_webview_window(&label) {
        existing
    } else {
        let url = format!("/?view=pin-toolbar&pin_id={pin_id}");
        tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
            .title("Acciones del recorte")
            .inner_size(330.0, 44.0)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(true)
            .resizable(false)
            .shadow(true)
            .visible(false)
            .build()
            .map_err(|e| format!("Fallo al abrir las acciones del recorte: {e}"))?
    };
    position_pin_toolbar(pin, &toolbar)?;
    toolbar.show().map_err(|e| e.to_string())?;
    Ok(toolbar)
}

fn position_pin_toolbar(
    pin: &tauri::WebviewWindow,
    toolbar: &tauri::WebviewWindow,
) -> Result<(), String> {
    let position = pin.outer_position().map_err(|e| e.to_string())?;
    let pin_size = pin.outer_size().map_err(|e| e.to_string())?;
    let size = toolbar.outer_size().map_err(|e| e.to_string())?;
    let mut x = position.x + (pin_size.width as i32 - size.width as i32) / 2;
    let below = position.y + pin_size.height as i32 + 8;
    let mut y = below;
    if let Some(monitor) = pin.current_monitor().map_err(|e| e.to_string())? {
        let left = monitor.position().x;
        let top = monitor.position().y;
        let right = left + monitor.size().width as i32;
        let bottom = top + monitor.size().height as i32;
        x = x.clamp(left, (right - size.width as i32).max(left));
        if below + size.height as i32 > bottom {
            y = (position.y - size.height as i32 - 8).max(top);
        }
    }
    toolbar
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

pub fn start_capture_flow(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<CaptureState>>();
    let Ok(_capture_guard) = state.capture_lock.try_lock() else {
        return Ok(());
    };
    if app
        .webview_windows()
        .keys()
        .any(|label| label.starts_with("overlay-"))
    {
        return Ok(());
    }
    let monitors = state.freeze_all_monitors()?;
    if monitors.is_empty() {
        return Err("No se encontraron pantallas disponibles".into());
    }

    close_all_overlays(app);

    for mon in &monitors {
        let label = format!("overlay-{}", mon.id);
        let url = format!("/?view=overlay&monitor_id={}", mon.id);

        let logical_x = mon.x as f64 / mon.scale_factor as f64;
        let logical_y = mon.y as f64 / mon.scale_factor as f64;
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
            .map_err(|e| {
                close_all_overlays(app);
                state.clear_frames();
                format!("Fallo al abrir overlay de captura: {e}")
            })?;

        let _ = win.set_focus();
        // Positions are physical on the desktop, including mixed-DPI monitor layouts.
        win.set_position(tauri::PhysicalPosition::new(mon.x, mon.y))
            .map_err(|e| e.to_string())?;
        win.set_size(tauri::PhysicalSize::new(mon.width, mon.height))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn run() {
    let capture_state = Arc::new(CaptureState::new());
    let state_for_protocol = Arc::clone(&capture_state);

    // Ruta de datos de persistencia local
    let data_dir = dirs_fallback_data_dir();
    let memory_db = Arc::new(
        MemoryDatabase::new(data_dir).expect("No se pudo abrir la memoria local de Screen-Pin"),
    );
    let db_for_protocol = Arc::clone(&memory_db);
    let db_for_gc = Arc::clone(&memory_db);

    tauri::Builder::default()
        .manage(capture_state)
        .manage(memory_db)
        .register_uri_scheme_protocol("pinasset", move |_app, request| {
            let path = request.uri().path();
            let trimmed = path.trim_start_matches('/');

            if let Some(mon_str) = trimmed.strip_prefix("monitor-") {
                if let Ok(mon_id) = mon_str.parse::<u32>()
                    && let Some(bytes) = state_for_protocol.get_monitor_preview(mon_id) {
                        return Response::builder()
                            .status(StatusCode::OK)
                            .header(CONTENT_TYPE, "image/jpeg")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(bytes)
                            .unwrap();
                }
            } else if uuid::Uuid::parse_str(trimmed).is_err() {
                return Response::builder().status(StatusCode::BAD_REQUEST).body(Vec::new()).unwrap();
            } else if let Some(bytes) = state_for_protocol.get_crop_bytes(trimmed) {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "image/png")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(bytes)
                    .unwrap();
            } else {
                // Si no está en RAM, intentar cargar desde el disco de Memoria
                let file_path = db_for_protocol.data_dir.join(format!("{}.png", trimmed));
                if let Ok(disk_bytes) = std::fs::read(&file_path) {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, "image/png")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(disk_bytes)
                        .unwrap();
                }
            }

            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new())
                .unwrap()
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            if let Some(main) = app.get_webview_window("main") {
                let main_handle = main.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_handle.hide();
                    }
                });
            }
            let mut shortcut_warnings = Vec::new();
            let gc_app = app.handle().clone();
            let gc_state = app.state::<Arc<CaptureState>>().inner().clone();
            std::thread::spawn(move || loop {
                if let Ok(paths) = db_for_gc.purge_expired_sensitive_pins() {
                    let changed = !paths.is_empty();
                    for path in paths {
                        if let Some(id) = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()) {
                            gc_state.release_crop(id);
                            if let Some(win) = gc_app.get_webview_window(&format!("pin-{id}")) { let _ = win.close(); }
                        }
                        let _ = std::fs::remove_file(db_for_gc.data_dir.join(path));
                    }
                    if changed { let _ = gc_app.emit("memory:changed", ()); }
                }
                std::thread::sleep(std::time::Duration::from_secs(60));
            });
            // Bandeja del sistema (Tray Icon)
            let quit_i = MenuItem::with_id(app, "quit", "Salir de Screen-Pin", true, None::<&str>)?;
            let memory_i = MenuItem::with_id(
                app,
                "memory",
                "Abrir Memoria",
                true,
                None::<&str>,
            )?;
            let capture_i = MenuItem::with_id(
                app,
                "capture",
                "Capturar Recorte",
                true,
                None::<&str>,
            )?;
            let restore_i = MenuItem::with_id(app, "restore", "Recuperar interacción", true, None::<&str>)?;
            let home_i = MenuItem::with_id(app, "home", "Abrir Screen-Pin", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&home_i, &capture_i, &memory_i, &restore_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("Icono de Screen-Pin").clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Screen-Pin - Recortes flotantes con memoria")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "capture" => {
                        run_window_action(app, start_capture_flow);
                    }
                    "memory" => {
                        run_window_action(app, show_memory_drawer);
                    }
                    "restore" => restore_interaction(app),
                    "home" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show(); let _ = win.set_focus();
                        }
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
                        run_window_action(app, start_capture_flow);
                    }
                })
                .build(app)?;

            // Atajos Globales: Alt+Shift+S (Captura) y Alt+Shift+M (Memoria)
            let capture_shortcut: Shortcut = "alt+shift+s".parse().map_err(|e| {
                tauri::Error::PluginInitialization("shortcut".to_string(), format!("Invalid shortcut: {e:?}"))
            })?;
            let memory_shortcut: Shortcut = "alt+shift+m".parse().map_err(|e| {
                tauri::Error::PluginInitialization("shortcut".to_string(), format!("Invalid shortcut: {e:?}"))
            })?;

            let app_handle_cap = app.handle().clone();
            if let Err(error) = app.global_shortcut().on_shortcut(capture_shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                        run_window_action(&app_handle_cap, start_capture_flow);
                }
            }) { eprintln!("No se pudo registrar Alt+Shift+S: {error}"); shortcut_warnings.push("capture".into()); }

            let app_handle_mem = app.handle().clone();
            if let Err(error) = app.global_shortcut().on_shortcut(memory_shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                        run_window_action(&app_handle_mem, show_memory_drawer);
                }
            }) { eprintln!("No se pudo registrar Alt+Shift+M: {error}"); shortcut_warnings.push("memory".into()); }

            let restore_shortcut: Shortcut = "alt+shift+g".parse()?;
            if let Err(error) = app.global_shortcut().on_shortcut(restore_shortcut, move |app, _, event| {
                if event.state() == ShortcutState::Pressed { restore_interaction(app); }
            }) { eprintln!("No se pudo registrar Alt+Shift+G: {error}"); shortcut_warnings.push("restore".into()); }
            app.manage(ShortcutWarnings(shortcut_warnings));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_monitors,
            get_shortcut_warnings,
            get_pin,
            confirm_crop,
            cancel_capture,
            trigger_capture,
            toggle_ghost_mode,
            get_memory_pins,
            search_memory_pins,
            archive_pin_to_memory,
            toggle_retention,
            repin_from_memory,
            open_memory_drawer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dirs_fallback_data_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os("SCREEN_PIN_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("APPDATA") {
        PathBuf::from(dir).join("ScreenPin")
    } else {
        PathBuf::from("./data")
    }
}
