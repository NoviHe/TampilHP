export interface AdbDevice {
  serial: string;
  state: string;
  transport: "usb" | "wifi";
  model: string;
  product: string;
  ip: string | null;
}

export interface WirelessSwitchResult {
  target_serial: string;
  ip: string;
  message: string;
}

export interface QrSessionInfo {
  service_name: string;
  password: string;
  qr_payload: string;
}

export interface QrPairEvent {
  status: "waiting" | "paired" | "connected" | "error";
  message: string;
  target_serial: string | null;
}

export interface DiscoveredAdbService {
  instance_name: string;
  ip: string;
  port: number;
}

export interface MirrorSettings {
  preset_id: string;
  max_size: number;
  max_fps: number;
  bitrate_mbps: number;
  video_codec: string;
  render_driver: string;
  audio_mode: string;
  audio_buffer_ms: number;
  video_buffer_ms: number;
  window_title: string;
  borderless: boolean;
  always_on_top: boolean;
  turn_screen_off: boolean;
  stay_awake: boolean;
  no_control: boolean;
  record_path: string;
}

export interface PresetDefinition {
  id: string;
  name: string;
  description: string;
  badge: string;
  settings: MirrorSettings;
}

export interface SavedDevice {
  id: string;
  alias: string;
  last_ip: string | null;
  last_serial: string;
}

export interface AppConfig {
  settings: MirrorSettings;
  saved_devices: SavedDevice[];
}

export interface DiagnosticItem {
  id: string;
  title: string;
  status: "ok" | "warn" | "error";
  detail: string;
  fix_action: string | null;
}

export interface ScrcpyLogEvent {
  serial: string;
  level: string;
  line: string;
}

export interface ScrcpyExitEvent {
  serial: string;
  code: number | null;
  hint: string | null;
}
