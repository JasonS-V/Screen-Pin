pub mod macos_ocr;
#[cfg(target_os = "windows")]
mod technical_ocr;
#[cfg(target_os = "windows")]
pub mod windows_ocr;

pub fn run_ocr(pin_id: &str, img: &image::RgbaImage) -> Result<OcrPayload, String> {
    #[cfg(target_os = "windows")]
    {
        windows_ocr::run_windows_ocr(pin_id, img)
    }
    #[cfg(target_os = "macos")]
    {
        macos_ocr::run_macos_ocr(pin_id, img)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (pin_id, img);
        Err("OCR no disponible en este sistema operativo".to_string())
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub bounds: NormalizedRect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrLine {
    pub words: Vec<OcrWord>,
    pub bounds: NormalizedRect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum SmartEntity {
    BankDigits { raw: String, clean_digits: String },
    WifiCredential { key: String },
    Checklist { items: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPayload {
    pub pin_id: String,
    pub lines: Vec<OcrLine>,
    pub full_text: String,
    pub entities: Vec<SmartEntity>,
    pub is_sensitive: bool,
}

/// Extrae secuencias bancarias eliminando espacios y guiones intermedios para que el
/// usuario pueda pegarlas directamente en formularios bancarios con límite de longitud.
pub fn extract_smart_entities(full_text: &str) -> (Vec<SmartEntity>, bool) {
    let mut entities = Vec::new();
    let mut is_sensitive = false;

    for line in full_text.lines() {
        let candidate_part = if let Some((_, value)) = line.split_once([':', '=']) {
            value.trim()
        } else {
            line.trim()
        };

        let digits_only: String = candidate_part.chars().filter(|c| c.is_ascii_digit()).collect();
        let allowed_formatting = candidate_part
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-' || *c == ' ')
            .count();

        let is_candidate_bank = (10..=24).contains(&digits_only.len())
            && !candidate_part.is_empty()
            && allowed_formatting * 10 >= candidate_part.chars().count() * 7;

        let line_digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
        let line_allowed = line
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-' || *c == ' ')
            .count();
        let is_line_bank = (10..=24).contains(&line_digits.len())
            && !line.is_empty()
            && line_allowed * 10 >= line.chars().count() * 7;

        // Evita falsos positivos en párrafos largos exigiendo que la línea o su valor sea mayormente numérico
        if is_candidate_bank {
            entities.push(SmartEntity::BankDigits {
                raw: line.trim().to_string(),
                clean_digits: digits_only,
            });
            is_sensitive = true;
        } else if is_line_bank {
            entities.push(SmartEntity::BankDigits {
                raw: line.trim().to_string(),
                clean_digits: line_digits,
            });
            is_sensitive = true;
        }

        // Detección de claves Wi-Fi o contraseñas
        if let Some((label, value)) = line.split_once([':', '=']) {
            let credential_label = label
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| {
                    [
                        "pass",
                        "password",
                        "clave",
                        "contraseña",
                        "contrasena",
                        "wpa",
                        "wpa2",
                    ]
                    .contains(&word)
                });
            let key = value.trim().to_string();
            if credential_label && !key.is_empty() {
                entities.push(SmartEntity::WifiCredential { key });
                is_sensitive = true;
            }
        }
    }

    (entities, is_sensitive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bank_account() {
        let sample = "Titular: CARLOS MENDOZA\nCuenta: 191-2849102-0-94\nCCI: 00219100284910209451";
        let (entities, is_sensitive) = extract_smart_entities(sample);

        assert!(is_sensitive);
        assert_eq!(entities.len(), 2);

        match &entities[0] {
            SmartEntity::BankDigits { clean_digits, .. } => {
                assert_eq!(clean_digits, "1912849102094");
            }
            _ => panic!("Entidad esperada: BankDigits"),
        }

        match &entities[1] {
            SmartEntity::BankDigits { clean_digits, .. } => {
                assert_eq!(clean_digits, "00219100284910209451");
            }
            _ => panic!("Entidad esperada: BankDigits"),
        }
    }

    #[test]
    fn test_extract_wifi_credential() {
        let sample = "Red: Invitados_5G\nPassword: MiClaveSecreta123";
        let (entities, is_sensitive) = extract_smart_entities(sample);

        assert!(is_sensitive);
        assert_eq!(entities.len(), 1);

        match &entities[0] {
            SmartEntity::WifiCredential { key } => {
                assert_eq!(key, "MiClaveSecreta123");
            }
            _ => panic!("Entidad esperada: WifiCredential"),
        }
    }

    #[test]
    fn test_plain_text_not_sensitive() {
        let sample = "Esta es una nota normal sobre compras de supermercado:\n- Manzanas\n- Leche";
        let (entities, is_sensitive) = extract_smart_entities(sample);

        assert!(!is_sensitive);
        assert!(entities.is_empty());
    }

    #[test]
    fn recognizes_spanish_credentials_without_matching_unrelated_words() {
        assert!(extract_smart_entities("Contraseña: MiClave2026").1);
        assert!(!extract_smart_entities("Compass: norte").1);
    }

    #[test]
    fn test_extract_bank_account_with_long_label() {
        let sample = "Número de cuenta bancaria: 191-2849102-0-94";
        let (entities, is_sensitive) = extract_smart_entities(sample);

        assert!(is_sensitive);
        assert_eq!(entities.len(), 1);
        match &entities[0] {
            SmartEntity::BankDigits { clean_digits, .. } => {
                assert_eq!(clean_digits, "1912849102094");
            }
            _ => panic!("Entidad esperada: BankDigits"),
        }
    }
}
