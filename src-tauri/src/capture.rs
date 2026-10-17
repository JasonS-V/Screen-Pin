use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use xcap::Monitor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropSelection {
    pub monitor_id: u32,
    pub physical_x: u32,
    pub physical_y: u32,
    pub physical_w: u32,
    pub physical_h: u32,
}

#[derive(Clone, Default)]
pub struct CaptureState {
    pub capture_lock: Arc<Mutex<()>>,
    /// Buffers completos capturados de cada monitor indexados por monitor.id()
    pub monitor_frames: Arc<Mutex<HashMap<u32, RgbaImage>>>,
    /// Previews codificados en JPEG para visualización ultra rápida e instantánea en el overlay
    pub monitor_previews: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
    /// Metadatos de cada monitor
    pub monitors: Arc<Mutex<Vec<MonitorInfo>>>,
    /// Recortes listos para ser consumidos por stickers via pinasset://
    pub pinned_crops: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl CaptureState {
    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        Ok(self.monitors.lock().map_err(|e| e.to_string())?.clone())
    }

    pub fn clear_frames(&self) {
        if let Ok(mut frames) = self.monitor_frames.lock() {
            frames.clear();
        }
        if let Ok(mut previews) = self.monitor_previews.lock() {
            previews.clear();
        }
    }

    pub fn release_crop(&self, id: &str) {
        if let Ok(mut crops) = self.pinned_crops.lock() {
            crops.remove(id);
        }
    }
    pub fn new() -> Self {
        Self {
            capture_lock: Arc::new(Mutex::new(())),
            monitor_frames: Arc::new(Mutex::new(HashMap::new())),
            monitor_previews: Arc::new(Mutex::new(HashMap::new())),
            monitors: Arc::new(Mutex::new(Vec::new())),
            pinned_crops: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Captura en paralelo todos los monitores activos en RAM antes de que cambie el foco de ventanas.
    pub fn freeze_all_monitors(&self) -> Result<Vec<MonitorInfo>, String> {
        let all_monitors = Monitor::all().map_err(|e| format!("Fallo al listar monitores: {e}"))?;
        let mut infos = Vec::new();
        let mut frames = HashMap::new();
        let mut previews = HashMap::new();

        for (idx, mon) in all_monitors.into_iter().enumerate() {
            let id = idx as u32;
            let name = mon.name().map_err(|e| e.to_string())?;
            let x = mon.x().map_err(|e| e.to_string())?;
            let y = mon.y().map_err(|e| e.to_string())?;
            let width = mon.width().map_err(|e| e.to_string())?;
            let height = mon.height().map_err(|e| e.to_string())?;
            let is_primary = mon.is_primary().map_err(|e| e.to_string())?;
            let scale_factor = mon.scale_factor().map_err(|e| e.to_string())?;

            let info = MonitorInfo {
                id,
                name,
                x,
                y,
                width,
                height,
                scale_factor,
                is_primary,
            };
            infos.push(info);

            let img = mon
                .capture_image()
                .map_err(|e| format!("Fallo al capturar monitor: {e}"))?;

            // Pre-codificar JPEG para visualización instantánea en overlay sin congelamientos
            let mut jpg_bytes = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut jpg_bytes);
            let rgb_img = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            if rgb_img
                .write_to(&mut cursor, image::ImageFormat::Jpeg)
                .is_ok()
            {
                previews.insert(id, jpg_bytes);
            }

            frames.insert(id, img);
        }

        let mut stored_frames = self.monitor_frames.lock().map_err(|e| e.to_string())?;
        *stored_frames = frames;

        let mut stored_previews = self.monitor_previews.lock().map_err(|e| e.to_string())?;
        *stored_previews = previews;

        let mut stored_monitors = self.monitors.lock().map_err(|e| e.to_string())?;
        *stored_monitors = infos.clone();

        Ok(infos)
    }

    pub fn get_monitor_preview(&self, monitor_id: u32) -> Option<Vec<u8>> {
        let previews = self.monitor_previews.lock().ok()?;
        previews.get(&monitor_id).cloned()
    }

    /// Extrae el rectángulo seleccionado del buffer en RAM y lo codifica a PNG en memoria
    pub fn crop_and_store(
        &self,
        crop_id: &str,
        crop: CropSelection,
    ) -> Result<(Vec<u8>, RgbaImage), String> {
        let frames = self.monitor_frames.lock().map_err(|e| e.to_string())?;
        let frame = frames
            .get(&crop.monitor_id)
            .ok_or_else(|| format!("Monitor {} no encontrado en captura", crop.monitor_id))?;

        let frame_w = frame.width();
        let frame_h = frame.height();

        let x = crop.physical_x;
        let y = crop.physical_y;
        if x >= frame_w || y >= frame_h || crop.physical_w == 0 || crop.physical_h == 0 {
            return Err("La selección está vacía o fuera de la pantalla".into());
        }
        let w = crop.physical_w.min(frame_w - x);
        let h = crop.physical_h.min(frame_h - y);

        let sub_img = image::imageops::crop_imm(frame, x, y, w, h).to_image();

        let mut png_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        sub_img
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Fallo al codificar PNG en memoria: {e}"))?;

        let mut pinned = self.pinned_crops.lock().map_err(|e| e.to_string())?;
        pinned.insert(crop_id.to_string(), png_bytes.clone());

        Ok((png_bytes, sub_img))
    }

    pub fn get_crop_bytes(&self, crop_id: &str) -> Option<Vec<u8>> {
        let pinned = self.pinned_crops.lock().ok()?;
        pinned.get(crop_id).cloned()
    }
}

/// Cálculo matemático de coordenadas físicas a partir de coordenadas lógicas y escala DPI
/// según fórmula del plan maestro:
/// X_físico = floor((X_lógico - X_origen) * S_DPI)
/// Y_físico = floor((Y_lógico - Y_origen) * S_DPI)
pub fn calculate_physical_coords(
    logical_x: f64,
    logical_y: f64,
    logical_w: f64,
    logical_h: f64,
    origin_x: f64,
    origin_y: f64,
    scale_factor: f64,
) -> (u32, u32, u32, u32) {
    let rel_x = (logical_x - origin_x).max(0.0);
    let rel_y = (logical_y - origin_y).max(0.0);

    let phys_x = (rel_x * scale_factor).floor() as u32;
    let phys_y = (rel_y * scale_factor).floor() as u32;
    let phys_w = (logical_w * scale_factor).floor().max(1.0) as u32;
    let phys_h = (logical_h * scale_factor).floor().max(1.0) as u32;

    (phys_x, phys_y, phys_w, phys_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn rejects_empty_and_out_of_bounds_crops() {
        let state = CaptureState::new();
        state
            .monitor_frames
            .lock()
            .unwrap()
            .insert(0, RgbaImage::new(10, 10));
        for (x, y, w, h) in [(10, 0, 2, 2), (0, 10, 2, 2), (0, 0, 0, 2)] {
            assert!(
                state
                    .crop_and_store(
                        "invalid",
                        CropSelection {
                            monitor_id: 0,
                            physical_x: x,
                            physical_y: y,
                            physical_w: w,
                            physical_h: h,
                        }
                    )
                    .is_err()
            );
        }
        assert!(state.get_crop_bytes("invalid").is_none());
    }

    #[test]
    fn test_coordinate_scaling_100_percent() {
        let (x, y, w, h) = calculate_physical_coords(100.0, 200.0, 400.0, 300.0, 0.0, 0.0, 1.0);
        assert_eq!((x, y, w, h), (100, 200, 400, 300));
    }

    #[test]
    fn test_coordinate_scaling_125_percent() {
        // En Windows con 125% DPI
        let (x, y, w, h) = calculate_physical_coords(100.0, 200.0, 400.0, 300.0, 0.0, 0.0, 1.25);
        assert_eq!((x, y, w, h), (125, 250, 500, 375));
    }

    #[test]
    fn test_coordinate_scaling_150_percent_with_monitor_offset() {
        // Monitor secundario en X=1920, Y=0 con 150% DPI
        let (x, y, w, h) = calculate_physical_coords(2020.0, 100.0, 300.0, 200.0, 1920.0, 0.0, 1.5);
        // rel_x = 100 -> 100 * 1.5 = 150
        // rel_y = 100 -> 100 * 1.5 = 150
        // w = 300 * 1.5 = 450
        // h = 200 * 1.5 = 300
        assert_eq!((x, y, w, h), (150, 150, 450, 300));
    }

    #[test]
    fn test_crop_and_store_in_memory() {
        let state = CaptureState::new();

        // Crear una imagen RGBA sintética de 100x100 píxeles
        let mut test_frame = RgbaImage::new(100, 100);
        for pixel in test_frame.pixels_mut() {
            *pixel = Rgba([255, 128, 0, 255]);
        }

        // Insertar imagen de prueba en el estado
        {
            let mut frames = state.monitor_frames.lock().unwrap();
            frames.insert(0, test_frame);
        }

        let crop = CropSelection {
            monitor_id: 0,
            physical_x: 10,
            physical_y: 20,
            physical_w: 30,
            physical_h: 40,
        };

        let result = state.crop_and_store("test-crop-1", crop);
        assert!(result.is_ok());

        let bytes = state.get_crop_bytes("test-crop-1");
        assert!(bytes.is_some());
        let png_bytes = bytes.unwrap();
        // Encabezado mágico de archivo PNG: [137, 80, 78, 71, 13, 10, 26, 10]
        assert_eq!(&png_bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
