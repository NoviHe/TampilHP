use crate::paths::app_config_path;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSettings {
    pub preset_id: String,
    pub max_size: u32,       // 0 = Native
    pub max_fps: u32,        // 0 = Unlimited
    pub bitrate_mbps: u32,
    pub video_codec: String, // "h264" | "h265" | "av1"
    pub render_driver: String, // "auto" | "direct3d" | "opengl" | "software"
    pub audio_mode: String,  // "output" | "playback" | "none"
    pub audio_buffer_ms: u32,
    pub video_buffer_ms: u32,
    pub window_title: String,
    pub borderless: bool,
    pub always_on_top: bool,
    pub turn_screen_off: bool,
    pub stay_awake: bool,
    pub no_control: bool,
    pub record_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub badge: String,
    pub settings: MirrorSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedDevice {
    pub id: String,
    pub alias: String,
    pub last_ip: Option<String>,
    pub last_serial: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub settings: MirrorSettings,
    pub saved_devices: Vec<SavedDevice>,
}

impl Default for MirrorSettings {
    fn default() -> Self {
        Self {
            preset_id: "gaming_usb".into(),
            max_size: 1080,
            max_fps: 60,
            bitrate_mbps: 16,
            video_codec: "h264".into(),
            render_driver: "direct3d".into(),
            audio_mode: "output".into(),
            audio_buffer_ms: 20,
            video_buffer_ms: 0,
            window_title: "TampilHP - Stream".into(),
            borderless: false,
            always_on_top: false,
            turn_screen_off: false,
            stay_awake: true,
            no_control: false,
            record_path: String::new(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: MirrorSettings::default(),
            saved_devices: Vec::new(),
        }
    }
}

pub fn builtin_presets() -> Vec<PresetDefinition> {
    vec![
        PresetDefinition {
            id: "gaming_usb".into(),
            name: "Gaming Ultra Low-Latency".into(),
            description: "Respons tercepat via USB, buffer 0 ms, 60 FPS stabil.".into(),
            badge: "USB / Low Latency".into(),
            settings: MirrorSettings {
                preset_id: "gaming_usb".into(),
                max_size: 1080,
                max_fps: 60,
                bitrate_mbps: 16,
                video_codec: "h264".into(),
                render_driver: "direct3d".into(),
                audio_mode: "output".into(),
                audio_buffer_ms: 20,
                video_buffer_ms: 0,
                window_title: "TampilHP - Gaming".into(),
                borderless: false,
                always_on_top: false,
                turn_screen_off: false,
                stay_awake: true,
                no_control: false,
                record_path: String::new(),
            },
        },
        PresetDefinition {
            id: "wireless_smooth".into(),
            name: "Wireless Stabil".into(),
            description: "Ringan & mulus di Wi-Fi 5GHz, layar HP mati agar hemat baterai.".into(),
            badge: "Wi-Fi".into(),
            settings: MirrorSettings {
                preset_id: "wireless_smooth".into(),
                max_size: 1280,
                max_fps: 60,
                bitrate_mbps: 8,
                video_codec: "h264".into(),
                render_driver: "direct3d".into(),
                audio_mode: "output".into(),
                audio_buffer_ms: 35,
                video_buffer_ms: 0,
                window_title: "TampilHP - Wireless".into(),
                borderless: false,
                always_on_top: false,
                turn_screen_off: true,
                stay_awake: true,
                no_control: false,
                record_path: String::new(),
            },
        },
        PresetDefinition {
            id: "obs_stream".into(),
            name: "OBS / Streamlabs".into(),
            description: "Judul window tetap agar Scene Window Capture tidak pernah putus.".into(),
            badge: "OBS / Streamlabs".into(),
            settings: MirrorSettings {
                preset_id: "obs_stream".into(),
                max_size: 1920,
                max_fps: 60,
                bitrate_mbps: 12,
                video_codec: "h264".into(),
                render_driver: "direct3d".into(),
                audio_mode: "output".into(),
                audio_buffer_ms: 30,
                video_buffer_ms: 0,
                window_title: "TampilHP - OBS Stream".into(),
                borderless: false,
                always_on_top: false,
                turn_screen_off: true,
                stay_awake: true,
                no_control: false,
                record_path: String::new(),
            },
        },
        PresetDefinition {
            id: "tiktok_studio".into(),
            name: "TikTok LIVE Studio".into(),
            description: "Pakai renderer OpenGL untuk mencegah black screen saat Window Capture.".into(),
            badge: "TikTok Studio".into(),
            settings: MirrorSettings {
                preset_id: "tiktok_studio".into(),
                max_size: 1080,
                max_fps: 60,
                bitrate_mbps: 10,
                video_codec: "h264".into(),
                render_driver: "opengl".into(),
                audio_mode: "output".into(),
                audio_buffer_ms: 30,
                video_buffer_ms: 0,
                window_title: "TampilHP - TikTok Live".into(),
                borderless: false,
                always_on_top: false,
                turn_screen_off: true,
                stay_awake: true,
                no_control: false,
                record_path: String::new(),
            },
        },
    ]
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let Ok(path) = app_config_path(app) else {
        return AppConfig::default();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return AppConfig::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_config(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = app_config_path(app)?;
    let raw = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Gagal serialize config: {e}"))?;
    std::fs::write(path, raw).map_err(|e| format!("Gagal simpan config: {e}"))
}
