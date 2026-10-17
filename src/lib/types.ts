export interface NormalizedRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OcrWord {
  text: string;
  bounds: NormalizedRect;
}

export interface OcrLine {
  words: OcrWord[];
  bounds: NormalizedRect;
}

export type SmartEntity =
  | { kind: 'BankDigits'; value: { raw: string; clean_digits: string } }
  | { kind: 'WifiCredential'; value: { key: string } }
  | { kind: 'Checklist'; value: { items: string[] } };

export interface OcrPayload {
  pin_id: string;
  lines: OcrLine[];
  full_text: string;
  entities: SmartEntity[];
  is_sensitive: boolean;
}

export interface MonitorInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  is_primary: boolean;
}

export interface CropSelection {
  monitor_id: number;
  physical_x: number;
  physical_y: number;
  physical_w: number;
  physical_h: number;
}

export interface PinRecord {
  id: string;
  created_at: number;
  archived_at: number | null;
  image_path: string;
  pos_x: number;
  pos_y: number;
  width: number;
  height: number;
  monitor_id: number;
  raw_text: string;
  ocr_json: string;
  entity_type: string | null;
  is_sensitive: boolean;
  expires_at: number | null;
  is_pinned_forever: boolean;
  ocr_status: 'pending' | 'ready' | 'error';
}
