pub mod capture;
pub mod memory_db;
pub mod ocr;

use capture::{CaptureState, CropSelection, MonitorInfo};
use memory_db::{MemoryDatabase, PinRecord};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    http::{header::CONTENT_TYPE, Response, StatusCode},
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
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
    db: State<'_, Arc<MemoryDatabase>>,
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

    let (png_bytes, sub_img) = state.crop_and_store(&crop_id, crop)?;

    // Guardar imagen en disco local en formato WebP / PNG
    let img_filename = format!("{}.png", crop_id);
    let img_path = db.data_dir.join(&img_filename);
    let _ = std::fs::write(&img_path, &png_bytes);

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
    };
    let _ = db.save_pin(&initial_pin);

    // Cerrar ventanas de overlay
    close_all_overlays(&app);

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
                    // Actualizar registro en base de datos con OCR y clasificación
                    let entity_kind = payload.entities.first().map(|e| match e {
                        ocr::SmartEntity::BankDigits { .. } => "BankDigits".to_string(),
                        ocr::SmartEntity::WifiCredential { .. } => "WifiCredential".to_string(),
                        ocr::SmartEntity::Checklist { .. } => "Checklist".to_string(),
                    });

                    let expires_at = if payload.is_sensitive {
                        Some(now + 1800) // 30 minutos de retención por privacidad
                    } else {
                        None
                    };

                    let updated_pin = PinRecord {
                        id: pin_id_for_ocr.clone(),
                        created_at: now,
                        archived_at: None,
                        image_path: format!("{}.png", pin_id_for_ocr),
                        pos_x: logical_x,
                        pos_y: logical_y,
                        width: logical_w,
                        height: logical_h,
                        monitor_id,
                        raw_text: payload.full_text.clone(),
                        ocr_json: serde_json::to_string(&payload.lines).unwrap_or_default(),
                        entity_type: entity_kind,
                        is_sensitive: payload.is_sensitive,
                        expires_at,
                        is_pinned_forever: false,
                    };
                    let _ = db_for_ocr.save_pin(&updated_pin);

                    let event_name = format!("ocr:ready:{}", pin_id_for_ocr);
                    let _ = app_for_ocr.emit(&event_name, payload);
                }
                Err(err) => {
                    eprintln!("Aviso: fallo en OCR para {pin_id_for_ocr}: {err}");
                    let event_name = format!("ocr:error:{}", pin_id_for_ocr);
                    let _ = app_for_ocr.emit(&event_name, ());
                }
            }
        }
    });

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
fn archive_pin_to_memory(
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<(), String> {
    db.archive_pin(&pin_id)
}

#[tauri::command]
fn toggle_retention(
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<bool, String> {
    db.toggle_pinned_forever(&pin_id)
}

#[tauri::command]
fn repin_from_memory(
    app: AppHandle,
    state: State<'_, Arc<CaptureState>>,
    db: State<'_, Arc<MemoryDatabase>>,
    pin_id: String,
) -> Result<(), String> {
    let pins = db.search_pins(&pin_id)?;
    let pin = pins
        .into_iter()
        .find(|p| p.id == pin_id)
        .ok_or_else(|| "Pin no encontrado en base de datos".to_string())?;

    // Asegurar que la imagen esté en el buffer de memoria pinasset://
    if state.get_crop_bytes(&pin.id).is_none() {
        let full_path = db.data_dir.join(&pin.image_path);
        if let Ok(bytes) = std::fs::read(&full_path) {
            let mut pinned = state.pinned_crops.lock().unwrap();
            pinned.insert(pin.id.clone(), bytes);
        }
    }

    spawn_pin_window(&app, &pin.id, pin.pos_x, pin.pos_y, pin.width, pin.height)?;

    Ok(())
}

#[tauri::command]
fn open_memory_drawer(app: AppHandle) -> Result<(), String> {
    let label = "memory-drawer";
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::App("/memory".into()))
        .title("Memoria - Screen-Pin")
        .inner_size(360.0, 680.0)
        .decorations(true)
        .always_on_top(true)
        .resizable(true)
        .build()
        .map_err(|e| format!("Fallo al abrir ventana de Memoria: {e}"))?;

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

    // Ruta de datos de persistencia local
    let data_dir = dirs_fallback_data_dir();
    let memory_db = Arc::new(
        MemoryDatabase::new(data_dir.clone())
            .unwrap_or_else(|_| MemoryDatabase::new_in_memory().unwrap()),
    );
    let db_for_protocol = Arc::clone(&memory_db);
    let db_for_gc = Arc::clone(&memory_db);

    // Demonio en segundo plano de limpieza de datos sensibles (cada 60 segundos)
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        if let Ok(paths) = db_for_gc.purge_expired_sensitive_pins() {
            for path in paths {
                let file_path = db_for_gc.data_dir.join(path);
                let _ = std::fs::remove_file(file_path);
            }
        }
    });

    tauri::Builder::default()
        .manage(capture_state)
        .manage(memory_db)
        .register_uri_scheme_protocol("pinasset", move |_app, request| {
            let path = request.uri().path();
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
            } else {
                // Si no está en RAM, intentar cargar desde el disco de Memoria
                let file_path = db_for_protocol.data_dir.join(format!("{}.png", trimmed));
                if let Ok(disk_bytes) = std::fs::read(&file_path) {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, "image/png")
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
            // Bandeja del sistema (Tray Icon)
            let quit_i = MenuItem::with_id(app, "quit", "Salir de Screen-Pin", true, None::<&str>)?;
            let memory_i = MenuItem::with_id(
                app,
                "memory",
                "Abrir Memoria (Alt+Shift+M)",
                true,
                None::<&str>,
            )?;
            let capture_i = MenuItem::with_id(
                app,
                "capture",
                "Capturar Recorte (Alt+Shift+S)",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&capture_i, &memory_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Screen-Pin - Recortes flotantes con memoria")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "capture" => {
                        let _ = start_capture_flow(app);
                    }
                    "memory" => {
                        let _ = open_memory_drawer(app.clone());
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

            // Atajos Globales: Alt+Shift+S (Captura) y Alt+Shift+M (Memoria)
            let capture_shortcut: Shortcut = "alt+shift+s".parse().map_err(|e| {
                tauri::Error::PluginInitialization("shortcut".to_string(), format!("Invalid shortcut: {e:?}"))
            })?;
            let memory_shortcut: Shortcut = "alt+shift+m".parse().map_err(|e| {
                tauri::Error::PluginInitialization("shortcut".to_string(), format!("Invalid shortcut: {e:?}"))
            })?;

            let app_handle_cap = app.handle().clone();
            app.global_shortcut().on_shortcut(capture_shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = start_capture_flow(&app_handle_cap);
                }
            })?;

            let app_handle_mem = app.handle().clone();
            app.global_shortcut().on_shortcut(memory_shortcut, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let _ = open_memory_drawer(app_handle_mem.clone());
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_monitors,
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
    if let Some(dir) = std::env::var_os("APPDATA") {
        PathBuf::from(dir).join("ScreenPin")
    } else {
        PathBuf::from("./data")
    }
}
