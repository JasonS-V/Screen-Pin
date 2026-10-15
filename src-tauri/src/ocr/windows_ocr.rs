#[cfg(target_os = "windows")]
use super::{extract_smart_entities, NormalizedRect, OcrLine, OcrPayload, OcrWord};
use image::RgbaImage;

#[cfg(target_os = "windows")]
pub fn run_windows_ocr(pin_id: &str, img: &RgbaImage) -> Result<OcrPayload, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    let width = img.width();
    let height = img.height();

    if width == 0 || height == 0 {
        return Err("Dimensiones de imagen inválidas para OCR".to_string());
    }

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("Fallo al inicializar Windows OCR: {e}"))?;

    // Crear buffer en memoria para Windows Runtime
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|e| format!("Fallo al crear stream de memoria: {e}"))?;
    let writer = DataWriter::CreateDataWriter(&stream)
        .map_err(|e| format!("Fallo al crear DataWriter: {e}"))?;

    let raw_bytes = img.as_raw();
    writer
        .WriteBytes(raw_bytes)
        .map_err(|e| format!("Fallo al escribir bytes: {e}"))?;
    let ibuffer = writer
        .DetachBuffer()
        .map_err(|e| format!("Fallo al desacoplar buffer: {e}"))?;

    let software_bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &ibuffer,
        BitmapPixelFormat::Rgba8,
        width as i32,
        height as i32,
    )
    .map_err(|e| format!("Fallo al crear SoftwareBitmap: {e}"))?;

    let ocr_op = engine
        .RecognizeAsync(&software_bitmap)
        .map_err(|e| format!("Fallo al iniciar reconocimiento: {e}"))?;

    let ocr_result = ocr_op
        .join()
        .map_err(|e| format!("Fallo en operación de OCR: {e}"))?;

    let mut lines = Vec::new();
    let mut full_text_lines = Vec::new();

    let ocr_lines = ocr_result
        .Lines()
        .map_err(|e| format!("Fallo al leer líneas OCR: {e}"))?;

    let total_lines = ocr_lines.Size().unwrap_or(0);

    for line_idx in 0..total_lines {
        if let Ok(line) = ocr_lines.GetAt(line_idx) {
            let line_text = line.Text().unwrap_or_default().to_string();
            full_text_lines.push(line_text);

            let mut words = Vec::new();
            if let Ok(ocr_words) = line.Words() {
                let total_words = ocr_words.Size().unwrap_or(0);
                for word_idx in 0..total_words {
                    if let Ok(word) = ocr_words.GetAt(word_idx) {
                        let text = word.Text().unwrap_or_default().to_string();
                        if let Ok(rect) = word.BoundingRect() {
                            let norm_rect = NormalizedRect {
                                x: (rect.X as f32 / width as f32).clamp(0.0, 1.0),
                                y: (rect.Y as f32 / height as f32).clamp(0.0, 1.0),
                                width: (rect.Width as f32 / width as f32).clamp(0.0, 1.0),
                                height: (rect.Height as f32 / height as f32).clamp(0.0, 1.0),
                            };
                            words.push(OcrWord {
                                text,
                                bounds: norm_rect,
                            });
                        }
                    }
                }
            }

            // Rectángulo englobante de la línea
            let line_norm_rect = if let Some(first) = words.first() {
                let min_x = words.iter().map(|w| w.bounds.x).fold(first.bounds.x, f32::min);
                let min_y = words.iter().map(|w| w.bounds.y).fold(first.bounds.y, f32::min);
                let max_x = words
                    .iter()
                    .map(|w| w.bounds.x + w.bounds.width)
                    .fold(first.bounds.x + first.bounds.width, f32::max);
                let max_y = words
                    .iter()
                    .map(|w| w.bounds.y + w.bounds.height)
                    .fold(first.bounds.y + first.bounds.height, f32::max);
                NormalizedRect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                }
            } else {
                NormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                }
            };

            lines.push(OcrLine {
                words,
                bounds: line_norm_rect,
            });
        }
    }

    let full_text = full_text_lines.join("\n");
    let (entities, is_sensitive) = extract_smart_entities(&full_text);

    Ok(OcrPayload {
        pin_id: pin_id.to_string(),
        lines,
        full_text,
        entities,
        is_sensitive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_ocr_runs_on_synthetic_bitmap() {
        let mut img = RgbaImage::new(100, 40);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }
        let result = run_windows_ocr("synthetic-test", &img);
        assert!(result.is_ok(), "Windows OCR debe ejecutarse con éxito: {:?}", result.err());
        let payload = result.unwrap();
        assert_eq!(payload.pin_id, "synthetic-test");
    }
}
