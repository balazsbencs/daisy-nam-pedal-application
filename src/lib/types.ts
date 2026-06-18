export interface ModelInfo {
  id:                  string;
  name:                string;
  size_bytes:          number;
  imported_at:         string;
  tone3000_id?:        string;
  tone3000_model_id?:  string;
}

export interface IrInfo {
  id:          string;
  name:        string;
  tap_count:   number;
  sample_rate: number;
  trimmed:     boolean;
  size_bytes:  number;
  imported_at: string;
}

export interface Preset {
  id:            string;
  name:          string;
  model_id:      string | null;
  ir_id:         string | null;
  input_gain:    number;   // 0.0..2.0
  output_volume: number;   // 0.0..1.0
  bypass:        boolean;
}

export interface Tone3000EmbeddedUser {
  username: string;
  url: string;
}

export interface Tone3000Tone {
  id: number;
  title: string;
  description: string | null;
  gear: string;
  user: Tone3000EmbeddedUser;
  downloads_count: number;
  favorites_count: number;
  url: string;
}

export interface Tone3000Model {
  id: number;
  tone_id: number;
  name: string;
  size: string | null;
  architecture_version: string;
  model_url: string;
}

export interface SearchResult {
  tones: Tone3000Tone[];
  total: number;
  page: number;
}

export interface AuthStatus {
  authenticated: boolean;
  username?: string;
  avatar_url?: string;
}

export interface ImageEntry {
  entry_type: "model" | "ir" | "preset";
  name:       string;
  size_bytes: number;
  offset:     number;
}

export interface ImageSummary {
  entries:         ImageEntry[];
  total_bytes:     number;
  partition_bytes: number;
  free_bytes:      number;
  image_path:      string;
}
