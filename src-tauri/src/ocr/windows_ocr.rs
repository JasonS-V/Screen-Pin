#[cfg(target_os = "windows")]
use super::{NormalizedRect, OcrLine, OcrPayload, OcrWord, extract_smart_entities};
use image::RgbaImage;
use windows::Media::Ocr::OcrEngine;

#[cfg(target_os = "windows")]
pub fn run_windows_ocr(pin_id: &str, img: &RgbaImage) -> Result<OcrPayload, String> {
    if img.width() == 0 || img.height() == 0 {
        return Err("Dimensiones de imagen inválidas para OCR".to_string());
    }

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("Fallo al inicializar Windows OCR: {e}"))?;
    let technical_engine = super::technical_ocr::create_engine()
        .inspect_err(|error| eprintln!("Aviso: {error}"))
        .ok();
    let max_dimension = OcrEngine::MaxImageDimension()
        .map_err(|e| format!("Fallo al consultar el límite de OCR: {e}"))?;
    let padding = 12;
    let scale = 3;
    let tile_size = (max_dimension / scale).saturating_sub(2 * padding).max(1);
    let overlap = (tile_size / 3).min(256);
    let stride = tile_size.saturating_sub(overlap).max(1);
    let mut lines = Vec::new();

    let mut y = 0;
    loop {
        let height = tile_size.min(img.height() - y);
        let mut x = 0;
        loop {
            let width = tile_size.min(img.width() - x);
            let tile = image::imageops::crop_imm(img, x, y, width, height).to_image();
            let enhanced = enhance_contrast(&tile);
            let mut tile_lines = Vec::new();
            // Windows conserva los idiomas instalados; Tesseract recupera signos de código.
            for (source, scale, technical_pass) in [
                (&tile, 3, false),
                (&enhanced, 3, false),
                (&enhanced, 2, true),
            ] {
                let scaled = image::imageops::resize(
                    source,
                    width * scale,
                    height * scale,
                    image::imageops::FilterType::Lanczos3,
                );
                let border = scale * padding;
                let mut prepared = RgbaImage::from_pixel(
                    scaled.width() + 2 * border,
                    scaled.height() + 2 * border,
                    *source.get_pixel(0, 0),
                );
                image::imageops::replace(&mut prepared, &scaled, border.into(), border.into());
                let recognized = if technical_pass {
                    let Some(technical_engine) = technical_engine.as_ref() else {
                        continue;
                    };
                    match super::technical_ocr::recognize(technical_engine, &prepared) {
                        Ok(lines) => lines,
                        Err(error) => {
                            eprintln!(
                                "Aviso: no se pudo completar la lectura de símbolos: {error}"
                            );
                            continue;
                        }
                    }
                } else {
                    recognize_bitmap(&engine, &prepared)?
                };
                for mut line in recognized {
                    if technical_pass && !super::technical_ocr::is_technical(&line) {
                        continue;
                    }
                    // La zona compartida permite descartar palabras cortadas por un borde interno.
                    line.words.retain(|word| {
                        let left =
                            word.bounds.x * prepared.width() as f32 / scale as f32 - padding as f32;
                        let top = word.bounds.y * prepared.height() as f32 / scale as f32
                            - padding as f32;
                        let right =
                            left + word.bounds.width * prepared.width() as f32 / scale as f32;
                        let bottom =
                            top + word.bounds.height * prepared.height() as f32 / scale as f32;
                        !word.text.trim().is_empty()
                            && (x == 0 || left > 4.0)
                            && (y == 0 || top > 4.0)
                            && (x + width == img.width() || right < width as f32 - 4.0)
                            && (y + height == img.height() || bottom < height as f32 - 4.0)
                    });
                    if line.words.is_empty() {
                        continue;
                    }
                    for word in &mut line.words {
                        let left = (x as f32
                            + word.bounds.x * prepared.width() as f32 / scale as f32
                            - padding as f32)
                            .clamp(0.0, img.width() as f32);
                        let top = (y as f32
                            + word.bounds.y * prepared.height() as f32 / scale as f32
                            - padding as f32)
                            .clamp(0.0, img.height() as f32);
                        let width = word.bounds.width * prepared.width() as f32 / scale as f32;
                        let height = word.bounds.height * prepared.height() as f32 / scale as f32;
                        word.bounds = NormalizedRect {
                            x: left / img.width() as f32,
                            y: top / img.height() as f32,
                            width: width.min(img.width() as f32 - left) / img.width() as f32,
                            height: height.min(img.height() as f32 - top) / img.height() as f32,
                        };
                    }
                    line.bounds = line_bounds(&line.words);
                    if technical_pass {
                        // El modelo adicional es inglés: conserva las letras de otros idiomas.
                        let has_other_letters = tile_lines.iter().any(|existing: &OcrLine| {
                            same_line(&existing.bounds, &line.bounds)
                                && existing.words.iter().any(|word| {
                                    word.text
                                        .chars()
                                        .any(|c| c.is_alphabetic() && !c.is_ascii())
                                })
                        });
                        if has_other_letters {
                            continue;
                        }
                        tile_lines.retain(|existing| !same_line(&existing.bounds, &line.bounds));
                    }
                    merge_line(&mut tile_lines, line);
                }
            }
            for line in tile_lines {
                merge_line(&mut lines, line);
            }
            if x + width == img.width() {
                break;
            }
            x += stride;
        }
        if y + height == img.height() {
            break;
        }
        y += stride;
    }

    lines.sort_by(|a, b| {
        a.bounds
            .y
            .total_cmp(&b.bounds.y)
            .then(a.bounds.x.total_cmp(&b.bounds.x))
    });
    for line in &mut lines {
        repair_line_urls(line);
    }
    let full_text = lines
        .iter()
        .map(|line| {
            line.words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let (entities, is_sensitive) = extract_smart_entities(&full_text);
    Ok(OcrPayload {
        pin_id: pin_id.to_string(),
        lines,
        full_text,
        entities,
        is_sensitive,
    })
}

fn repair_line_urls(line: &mut OcrLine) {
    if line.words.is_empty() {
        return;
    }
    if line.words.len() == 1 {
        if let Some(word) = line.words.first_mut() {
            fix_google_doc_url(&mut word.text);
        }
        return;
    }

    let mut merged_words: Vec<OcrWord> = Vec::new();
    let mut i = 0;
    while i < line.words.len() {
        let current = &line.words[i];
        if is_url_start(&current.text) {
            let mut combined_text = current.text.clone();
            let mut combined_bounds = current.bounds.clone();
            let mut j = i + 1;
            while j < line.words.len() {
                let candidate = &line.words[j];
                if is_url_continuation(&combined_text, &candidate.text) {
                    combined_text.push_str(&candidate.text);
                    let right = (combined_bounds.x + combined_bounds.width)
                        .max(candidate.bounds.x + candidate.bounds.width);
                    let bottom = (combined_bounds.y + combined_bounds.height)
                        .max(candidate.bounds.y + candidate.bounds.height);
                    combined_bounds.x = combined_bounds.x.min(candidate.bounds.x);
                    combined_bounds.y = combined_bounds.y.min(candidate.bounds.y);
                    combined_bounds.width = right - combined_bounds.x;
                    combined_bounds.height = bottom - combined_bounds.y;
                    j += 1;
                } else {
                    break;
                }
            }
            fix_google_doc_url(&mut combined_text);
            merged_words.push(OcrWord {
                text: combined_text,
                bounds: combined_bounds,
            });
            i = j;
        } else {
            merged_words.push(current.clone());
            i += 1;
        }
    }
    line.words = merged_words;
}

fn is_url_start(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("www.")
        || lower.starts_with("docs.google.")
        || lower.starts_with("drive.google.")
        || lower.contains(".com/")
        || lower.contains(".org/")
        || lower.contains(".net/")
        || lower.contains(".edu/")
        || lower.contains(".gov/")
        || lower.contains(".io/")
        || lower.contains(".app/")
        || lower.contains(".dev/")
        || lower.contains(".me/")
}

fn is_url_continuation(current_url: &str, candidate: &str) -> bool {
    if candidate.is_empty() {
        return false;
    }
    if current_url.ends_with(['/', '=', '?', '&', '-']) {
        return true;
    }
    if candidate.starts_with(['/', '?', '&', '=', '#']) {
        return true;
    }
    if (current_url.contains("/d/")
        || current_url.contains("/file/")
        || current_url.contains("/document/")
        || current_url.contains("/spreadsheets/"))
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/')
    {
        return true;
    }
    if candidate.contains('/')
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/-_.~?&=%#".contains(c))
    {
        return true;
    }
    false
}

fn fix_google_doc_url(url: &mut String) {
    if let Some(pos) = url.find("/d/l") {
        let after = pos + 4;
        if after < url.len()
            && url[after..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            url.replace_range(pos..pos + 4, "/d/1");
        }
    } else if let Some(pos) = url.find("/d/I") {
        let after = pos + 4;
        if after < url.len()
            && url[after..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            url.replace_range(pos..pos + 4, "/d/1");
        }
    }
}

fn enhance_contrast(img: &RgbaImage) -> RgbaImage {
    let gray = image::imageops::grayscale(img);
    let mut enhanced = RgbaImage::from_pixel(img.width(), img.height(), image::Rgba([255; 4]));
    // El contraste local evita que un panel claro o un icono oculte el texto tenue cercano.
    for y in (0..img.height()).step_by(96) {
        for x in (0..img.width()).step_by(96) {
            let region = image::imageops::crop_imm(
                &gray,
                x,
                y,
                96.min(img.width() - x),
                96.min(img.height() - y),
            )
            .to_image();
            let min = region.pixels().map(|p| p[0]).min().unwrap_or(0);
            let max = region.pixels().map(|p| p[0]).max().unwrap_or(255);
            if max.saturating_sub(min) < 8 {
                continue;
            }
            let midpoint = (u16::from(min) + u16::from(max)) / 2;
            let dark_background = region
                .pixels()
                .filter(|p| u16::from(p[0]) < midpoint)
                .count()
                > region.len() / 2;
            for (dx, dy, pixel) in region.enumerate_pixels() {
                let value = (u16::from(pixel[0] - min) * 255 / u16::from(max - min)) as u8;
                let value = if dark_background { 255 - value } else { value };
                enhanced.put_pixel(x + dx, y + dy, image::Rgba([value, value, value, 255]));
            }
        }
    }
    enhanced
}

fn line_bounds(words: &[OcrWord]) -> NormalizedRect {
    let x = words.iter().map(|w| w.bounds.x).fold(1.0, f32::min);
    let y = words.iter().map(|w| w.bounds.y).fold(1.0, f32::min);
    let right = words
        .iter()
        .map(|w| w.bounds.x + w.bounds.width)
        .fold(0.0, f32::max);
    let bottom = words
        .iter()
        .map(|w| w.bounds.y + w.bounds.height)
        .fold(0.0, f32::max);
    NormalizedRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

fn merge_line(lines: &mut Vec<OcrLine>, mut incoming: OcrLine) {
    if incoming.words.is_empty() {
        return;
    }
    let matching = lines
        .iter_mut()
        .find(|line| same_line(&line.bounds, &incoming.bounds));
    if let Some(line) = matching {
        for word in incoming.words {
            let duplicate = line.words.iter().any(|existing| {
                let a = &existing.bounds;
                let b = &word.bounds;
                let width = ((a.x + a.width).min(b.x + b.width) - a.x.max(b.x)).max(0.0);
                let height = ((a.y + a.height).min(b.y + b.height) - a.y.max(b.y)).max(0.0);
                width * height > (a.width * a.height).min(b.width * b.height) * 0.5
            });
            if !duplicate {
                line.words.push(word);
            }
        }
        line.words.sort_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x));
        line.bounds = line_bounds(&line.words);
    } else {
        incoming
            .words
            .sort_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x));
        lines.push(incoming);
    }
}

fn same_line(a: &NormalizedRect, b: &NormalizedRect) -> bool {
    let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    overlap_y > a.height.min(b.height) * 0.5 && overlap_x > 0.0
}

fn recognize_bitmap(engine: &OcrEngine, img: &RgbaImage) -> Result<Vec<OcrLine>, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    let width = img.width();
    let height = img.height();

    if width == 0 || height == 0 {
        return Err("Dimensiones de imagen inválidas para OCR".to_string());
    }

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

    let ocr_lines = ocr_result
        .Lines()
        .map_err(|e| format!("Fallo al leer líneas OCR: {e}"))?;

    let total_lines = ocr_lines.Size().unwrap_or(0);

    for line_idx in 0..total_lines {
        if let Ok(line) = ocr_lines.GetAt(line_idx) {
            let mut words = Vec::new();
            if let Ok(ocr_words) = line.Words() {
                let total_words = ocr_words.Size().unwrap_or(0);
                for word_idx in 0..total_words {
                    if let Ok(word) = ocr_words.GetAt(word_idx) {
                        let text = word.Text().unwrap_or_default().to_string();
                        if let Ok(rect) = word.BoundingRect() {
                            let norm_rect = NormalizedRect {
                                x: (rect.X / width as f32).clamp(0.0, 1.0),
                                y: (rect.Y / height as f32).clamp(0.0, 1.0),
                                width: (rect.Width / width as f32).clamp(0.0, 1.0),
                                height: (rect.Height / height as f32).clamp(0.0, 1.0),
                            };
                            words.push(OcrWord {
                                text,
                                bounds: norm_rect,
                            });
                        }
                    }
                }
            }

            if !words.is_empty() {
                lines.push(OcrLine {
                    bounds: line_bounds(&words),
                    words,
                });
            }
        }
    }

    Ok(lines)
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
        assert!(
            result.is_ok(),
            "Windows OCR debe ejecutarse con éxito: {:?}",
            result.err()
        );
        let payload = result.unwrap();
        assert_eq!(payload.pin_id, "synthetic-test");
        assert!(payload.full_text.is_empty());
    }

    #[test]
    fn reads_small_dark_gray_text_numbers_and_punctuation() {
        let img = image::load_from_memory(include_bytes!("fixtures/dark-gray-text.png"))
            .unwrap()
            .to_rgba8();
        let payload = run_windows_ocr("dark-gray", &img).unwrap();
        assert!(payload.full_text.contains("12345678901234567890"));
        assert!(payload.full_text.contains(
            "Este es un texto largo para comprobar que se conserva completo hasta la palabra FINAL."
        ));
        assert!(payload.full_text.contains("correo@ejemplo.com"));
        assert!(payload.full_text.contains("1,234.56"));
        assert!(payload.full_text.contains("S/"));
        assert!(payload.full_text.contains("(50%)"));
        assert_eq!(payload.full_text.matches("FINAL").count(), 1);
        let number = payload
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .find(|word| word.text == "12345678901234567890")
            .unwrap();
        assert!((number.bounds.x * img.width() as f32 - 75.0).abs() < 10.0);
        assert!((number.bounds.y * img.height() as f32 - 18.0).abs() < 5.0);
    }

    #[test]
    fn accepts_captures_wider_than_the_engine_limit() {
        let max = windows::Media::Ocr::OcrEngine::MaxImageDimension().unwrap();
        let img = RgbaImage::from_pixel(max + 1, 40, image::Rgba([255, 255, 255, 255]));
        assert!(run_windows_ocr("wide", &img).is_ok());
    }

    #[test]
    fn preserves_numbers_across_tile_boundaries_without_duplicates() {
        let max = OcrEngine::MaxImageDimension().unwrap();
        let tile_size = max / 3 - 24;
        let fixture = image::load_from_memory(include_bytes!("fixtures/dark-gray-text.png"))
            .unwrap()
            .to_rgba8();
        let row = image::imageops::crop_imm(&fixture, 0, 0, 300, 40).to_image();
        let mut img = RgbaImage::from_pixel(max + 400, 80, image::Rgba([32, 32, 32, 255]));
        image::imageops::replace(&mut img, &row, (tile_size - 130).into(), 0);
        image::imageops::replace(&mut img, &row, (max - 130).into(), 0);
        let payload = run_windows_ocr("tile-boundaries", &img).unwrap();
        assert_eq!(payload.full_text.matches("12345678901234567890").count(), 2);
        assert_eq!(payload.full_text.matches("Referencia:").count(), 2);
        for word in payload.lines.iter().flat_map(|line| &line.words) {
            assert!(word.bounds.x >= 0.0 && word.bounds.x + word.bounds.width <= 1.001);
            assert!(word.bounds.y >= 0.0 && word.bounds.y + word.bounds.height <= 1.001);
        }
    }

    #[test]
    fn reads_gray_numbers_alongside_bright_content() {
        let mut img = image::load_from_memory(include_bytes!("fixtures/dark-gray-text.png"))
            .unwrap()
            .to_rgba8();
        let bright = RgbaImage::from_pixel(40, 20, image::Rgba([255, 255, 255, 255]));
        image::imageops::replace(&mut img, &bright, 900, 140);
        let payload = run_windows_ocr("mixed-contrast", &img).unwrap();
        assert!(
            payload.full_text.contains("12345678901234567890"),
            "{}",
            payload.full_text
        );
    }

    #[test]
    fn reads_dark_text_on_a_light_background() {
        let mut img = image::load_from_memory(include_bytes!("fixtures/dark-gray-text.png"))
            .unwrap()
            .to_rgba8();
        image::imageops::invert(&mut img);
        let payload = run_windows_ocr("light-background", &img).unwrap();
        assert!(payload.full_text.contains("12345678901234567890"));
        assert!(payload.full_text.contains("FINAL"));
    }

    #[test]
    fn preserves_text_and_coordinates_across_vertical_tiles() {
        let tile_size = OcrEngine::MaxImageDimension().unwrap() / 3 - 24;
        let fixture = image::load_from_memory(include_bytes!("fixtures/dark-gray-text.png"))
            .unwrap()
            .to_rgba8();
        let row = image::imageops::crop_imm(&fixture, 0, 0, 300, 40).to_image();
        let offset_y = tile_size - 25;
        let mut img = RgbaImage::from_pixel(320, tile_size + 80, image::Rgba([32, 32, 32, 255]));
        image::imageops::replace(&mut img, &row, 0, offset_y.into());
        let payload = run_windows_ocr("vertical-tiles", &img).unwrap();
        assert_eq!(payload.full_text.matches("12345678901234567890").count(), 1);
        let number = payload
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .find(|word| word.text == "12345678901234567890")
            .unwrap();
        assert!((number.bounds.y * img.height() as f32 - (offset_y + 18) as f32).abs() < 5.0);
    }

    #[test]
    fn rejects_empty_images() {
        for (width, height) in [(0, 0), (0, 10), (10, 0)] {
            assert!(run_windows_ocr("empty", &RgbaImage::new(width, height)).is_err());
        }
    }

    #[test]
    fn preserves_console_paths_operators_and_punctuation() {
        let img = image::load_from_memory(include_bytes!("fixtures/console-text.png"))
            .unwrap()
            .to_rgba8();
        let payload = run_windows_ocr("console", &img).unwrap();
        let prompt = payload
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .find(|word| word.text == r"C:\Users\jason>")
            .unwrap();
        assert!((prompt.bounds.x * img.width() as f32 - 12.0).abs() < 6.0);
        assert!((prompt.bounds.y * img.height() as f32 - 19.0).abs() < 6.0);
        for expected in [
            r"C:\Users\jason>",
            r#"echo "hola" | findstr /i "ok""#,
            "<=",
            "&&",
            "!=",
            "{",
            "[x, y]",
            "}",
            "foo_bar",
            "--short",
            "%PATH%",
            "$HOME",
            "#",
        ] {
            assert!(
                payload.full_text.contains(expected),
                "Missing {expected}: {}",
                payload.full_text
            );
        }
    }

    #[test]
    fn reads_console_symbols_on_a_light_background() {
        let mut img = image::load_from_memory(include_bytes!("fixtures/console-text.png"))
            .unwrap()
            .to_rgba8();
        image::imageops::invert(&mut img);
        let payload = run_windows_ocr("console-light", &img).unwrap();
        for expected in [
            r"C:\Users\jason>",
            "<=",
            "!=",
            "&&",
            "foo_bar",
            "%PATH%",
            "$HOME",
        ] {
            assert!(
                payload.full_text.contains(expected),
                "Missing {expected}: {}",
                payload.full_text
            );
        }
    }

    #[test]
    fn reads_low_contrast_console_symbols() {
        let mut img = image::load_from_memory(include_bytes!("fixtures/console-text.png"))
            .unwrap()
            .to_rgba8();
        for pixel in img.pixels_mut() {
            for value in &mut pixel.0[..3] {
                *value = 32 + (u16::from(*value - 12) * 30 / 192) as u8;
            }
        }
        let payload = run_windows_ocr("console-gray", &img).unwrap();
        for expected in [
            r"C:\Users\jason>",
            "<=",
            "!=",
            "&&",
            "foo_bar",
            "%PATH%",
            "$HOME",
        ] {
            assert!(
                payload.full_text.contains(expected),
                "Missing {expected}: {}",
                payload.full_text
            );
        }
    }

    #[test]
    fn reads_sheet_url() {
        let img = image::load_from_memory(include_bytes!("fixtures/sheet-url.png"))
            .unwrap()
            .to_rgba8();
        let payload = run_windows_ocr("sheet-url", &img).unwrap();
        assert!(
            payload.full_text.starts_with("docs.google.com/spreadsheets/d/1DRt5V6mPsxzMfzQsxW_Jf4"),
            "URL prefix should match accurately: {}",
            payload.full_text
        );
        assert!(
            payload.full_text.ends_with("/edit"),
            "URL should end with /edit: {}",
            payload.full_text
        );
        assert!(
            !payload.full_text.contains(' '),
            "URL should not contain spaces: {}",
            payload.full_text
        );
    }

    #[test]
    fn keeps_spanish_accents_alongside_technical_text() {
        let img = image::load_from_memory(include_bytes!("fixtures/spanish-text.png"))
            .unwrap()
            .to_rgba8();
        let payload = run_windows_ocr("spanish", &img).unwrap();
        for expected in [
            "Configuración",
            "tamaño",
            "año",
            "acción",
            "pingüino",
            "contraseña",
        ] {
            assert!(
                payload.full_text.contains(expected),
                "Missing {expected}: {}",
                payload.full_text
            );
        }
    }
}
