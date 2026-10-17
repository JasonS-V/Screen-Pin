use super::{NormalizedRect, OcrLine, OcrWord};
use image::RgbaImage;
use tesseract_rs::{TessPageSegMode, TesseractAPI};

pub(super) fn create_engine() -> Result<TesseractAPI, String> {
    let engine = TesseractAPI::new();
    let model = include_bytes!("../../resources/ocr/eng.traineddata");
    engine
        .init_5(model, model.len() as i32, "eng", 1, &[])
        .map_err(|e| format!("No se pudo iniciar la lectura de símbolos: {e}"))?;
    engine
        .set_page_seg_mode(TessPageSegMode::PSM_SINGLE_BLOCK)
        .map_err(|e| e.to_string())?;
    Ok(engine)
}

pub(super) fn recognize(engine: &TesseractAPI, img: &RgbaImage) -> Result<Vec<OcrLine>, String> {
    let gray = image::imageops::grayscale(img);
    engine
        .set_image(
            gray.as_raw(),
            gray.width() as i32,
            gray.height() as i32,
            1,
            gray.width() as i32,
        )
        .map_err(|e| e.to_string())?;
    let tsv = engine.get_tsv_text(0).map_err(|e| e.to_string())?;
    parse_tsv(&tsv, img.width(), img.height())
}

pub(super) fn is_technical(line: &OcrLine) -> bool {
    let text = line
        .words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    text.contains('\\')
        || text
            .chars()
            .filter(|c| "{}[]<>|=&_!$#`/^*".contains(*c))
            .count()
            >= 2
        || text.matches('%').count() >= 2
        || text.contains("--")
        || matches!(text.trim(), "{" | "}" | "[" | "]" | "|" | ">" | "$")
        || text.trim_start().starts_with('#')
}

fn parse_tsv(tsv: &str, width: u32, height: u32) -> Result<Vec<OcrLine>, String> {
    let mut lines: Vec<OcrLine> = Vec::new();
    for row in tsv.lines().skip(1) {
        let fields = row.splitn(12, '\t').collect::<Vec<_>>();
        if fields.len() != 12 {
            return Err("Resultado de lectura de símbolos inválido".into());
        }
        if fields[0] != "4" && fields[0] != "5" {
            continue;
        }
        let number = |index: usize| {
            fields[index]
                .parse::<u32>()
                .map_err(|e| format!("Coordenadas de lectura inválidas: {e}"))
        };
        let left = (number(6)? as f32 / width as f32).clamp(0.0, 1.0);
        let top = (number(7)? as f32 / height as f32).clamp(0.0, 1.0);
        let bounds = NormalizedRect {
            x: left,
            y: top,
            width: (number(8)? as f32 / width as f32).min(1.0 - left),
            height: (number(9)? as f32 / height as f32).min(1.0 - top),
        };
        if fields[0] == "4" {
            lines.push(OcrLine {
                bounds,
                words: Vec::new(),
            });
        } else if !fields[11].trim().is_empty() {
            let line = lines.last_mut().ok_or("Palabra sin línea de lectura")?;
            line.words.push(OcrWord {
                text: fields[11].to_string(),
                bounds,
            });
        }
    }
    lines.retain(|line| !line.words.is_empty());
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_symbols_unicode_and_coordinates_from_tesseract() {
        let tsv = concat!(
            "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n",
            "4\t1\t1\t1\t1\t0\t10\t20\t140\t20\t-1\t\n",
            "5\t1\t1\t1\t1\t1\t10\t20\t140\t20\t95.5\tC:\\José>\n",
            "4\t1\t1\t1\t2\t0\t10\t60\t20\t20\t-1\t\n",
            "5\t1\t1\t1\t2\t1\t10\t60\t20\t20\t90.0\t&&\n",
        );
        let lines = parse_tsv(tsv, 200, 100).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].words[0].text, "C:\\José>");
        assert_eq!(lines[1].words[0].text, "&&");
        assert_eq!(
            lines[0].words[0].bounds,
            NormalizedRect {
                x: 0.05,
                y: 0.2,
                width: 0.7,
                height: 0.2
            }
        );
    }

    #[test]
    fn rejects_malformed_ocr_metadata() {
        assert!(parse_tsv("header\n5\tbroken", 200, 100).is_err());
        assert!(
            parse_tsv(
                "header\n5\t1\t1\t1\t1\t1\tbad\t20\t140\t20\t95\ttext",
                200,
                100
            )
            .is_err()
        );
    }
}
