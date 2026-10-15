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
        let digits_only: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
        let allowed_formatting = line
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-' || *c == ' ')
            .count();

        // Evita falsos positivos en párrafos largos exigiendo que la línea sea mayormente numérica
        if (10..=24).contains(&digits_only.len()) && allowed_formatting * 10 >= line.len() * 7 {
            entities.push(SmartEntity::BankDigits {
                raw: line.trim().to_string(),
                clean_digits: digits_only,
            });
            is_sensitive = true;
        }

        // Detección de claves Wi-Fi o contraseñas
        let lower = line.to_lowercase();
        if lower.contains("pass") || lower.contains("clave") || lower.contains("password") || lower.contains("wpa") {
            if let Some(pos) = line.find(':') {
                let key = line[pos + 1..].trim().to_string();
                if !key.is_empty() {
                    entities.push(SmartEntity::WifiCredential { key });
                    is_sensitive = true;
                }
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
}
