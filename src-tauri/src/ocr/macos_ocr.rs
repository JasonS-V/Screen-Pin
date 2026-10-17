use super::{NormalizedRect, OcrLine, OcrPayload, OcrWord, extract_smart_entities};
#[cfg(target_os = "macos")]
use image::RgbaImage;

#[derive(serde::Deserialize)]
struct VisionResult {
    lines: Vec<VisionLine>,
}

#[derive(serde::Deserialize)]
struct VisionLine {
    text: String,
    words: Vec<OcrWord>,
    bounds: NormalizedRect,
}

pub fn parse_vision_json(json_str: &str, pin_id: &str) -> Result<OcrPayload, String> {
    let vision_res: VisionResult = serde_json::from_str(json_str)
        .map_err(|e| format!("Error al parsear respuesta de Vision: {e}"))?;

    let mut vision_lines = vision_res.lines;
    vision_lines.sort_by(|a, b| {
        a.bounds
            .y
            .total_cmp(&b.bounds.y)
            .then(a.bounds.x.total_cmp(&b.bounds.x))
    });

    let full_text = vision_lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let lines = vision_lines
        .into_iter()
        .map(|line| OcrLine {
            words: line.words,
            bounds: line.bounds,
        })
        .collect();

    let (entities, is_sensitive) = extract_smart_entities(&full_text);

    Ok(OcrPayload {
        pin_id: pin_id.to_string(),
        lines,
        full_text,
        entities,
        is_sensitive,
    })
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn run_macos_vision_ocr(
        image_bytes: *const u8,
        length: usize,
        out_json: *mut *mut std::os::raw::c_char,
    ) -> std::os::raw::c_int;

    fn free_macos_vision_ocr_result(ptr: *mut std::os::raw::c_char);
}

#[cfg(target_os = "macos")]
pub fn run_macos_ocr(pin_id: &str, img: &RgbaImage) -> Result<OcrPayload, String> {
    if img.width() == 0 || img.height() == 0 {
        return Err("Dimensiones de imagen inválidas para OCR".to_string());
    }

    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Error al codificar imagen para Vision OCR: {e}"))?;

    let mut out_json_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe {
        run_macos_vision_ocr(
            png_bytes.as_ptr(),
            png_bytes.len(),
            &mut out_json_ptr as *mut _,
        )
    };

    if code != 0 || out_json_ptr.is_null() {
        return Err(format!("macOS Vision OCR falló con código {code}"));
    }

    let json_str = unsafe {
        let c_str = std::ffi::CStr::from_ptr(out_json_ptr);
        let s = c_str.to_string_lossy().to_string();
        free_macos_vision_ocr_result(out_json_ptr);
        s
    };

    parse_vision_json(&json_str, pin_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vision_json_detects_lines_and_entities() {
        let sample_json = r#"{
            "lines": [
                {
                    "text": "Cuenta: 191-2849102-0-94",
                    "bounds": { "x": 0.05, "y": 0.1, "width": 0.8, "height": 0.08 },
                    "words": [
                        { "text": "Cuenta", "bounds": { "x": 0.05, "y": 0.1, "width": 0.2, "height": 0.08 } },
                        { "text": "1912849102094", "bounds": { "x": 0.3, "y": 0.1, "width": 0.5, "height": 0.08 } }
                    ]
                },
                {
                    "text": "Password: Secret123",
                    "bounds": { "x": 0.05, "y": 0.25, "width": 0.6, "height": 0.08 },
                    "words": [
                        { "text": "Password", "bounds": { "x": 0.05, "y": 0.25, "width": 0.25, "height": 0.08 } },
                        { "text": "Secret123", "bounds": { "x": 0.35, "y": 0.25, "width": 0.3, "height": 0.08 } }
                    ]
                }
            ]
        }"#;

        let payload = parse_vision_json(sample_json, "test-pin-1").unwrap();
        assert_eq!(payload.pin_id, "test-pin-1");
        assert_eq!(payload.lines.len(), 2);
        assert_eq!(
            payload.full_text,
            "Cuenta: 191-2849102-0-94\nPassword: Secret123"
        );
        assert!(payload.is_sensitive);
        assert_eq!(payload.entities.len(), 2);
    }

    #[test]
    fn test_parse_vision_json_sorts_lines_top_to_bottom() {
        let sample_json = r#"{
            "lines": [
                {
                    "text": "Abajo",
                    "bounds": { "x": 0.05, "y": 0.6, "width": 0.5, "height": 0.05 },
                    "words": [{ "text": "Abajo", "bounds": { "x": 0.05, "y": 0.6, "width": 0.5, "height": 0.05 } }]
                },
                {
                    "text": "Arriba",
                    "bounds": { "x": 0.05, "y": 0.1, "width": 0.5, "height": 0.05 },
                    "words": [{ "text": "Arriba", "bounds": { "x": 0.05, "y": 0.1, "width": 0.5, "height": 0.05 } }]
                }
            ]
        }"#;

        let payload = parse_vision_json(sample_json, "test-sort").unwrap();
        assert_eq!(payload.lines[0].words[0].text, "Arriba");
        assert_eq!(payload.lines[1].words[0].text, "Abajo");
        assert_eq!(payload.full_text, "Arriba\nAbajo");
    }
}
