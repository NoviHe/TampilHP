use crate::paths::{adb_path, resolve_scrcpy_dir, scrcpy_path};
use crate::presets::MirrorSettings;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct ScrcpyManager {
    pub sessions: Arc<Mutex<HashMap<String, Child>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScrcpyLogEvent {
    pub serial: String,
    pub level: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScrcpyExitEvent {
    pub serial: String,
    pub code: Option<i32>,
    pub hint: Option<String>,
}

pub fn build_scrcpy_args(serial: &str, s: &MirrorSettings) -> Vec<String> {
    let mut args = vec!["-s".to_string(), serial.to_string()];

    let title = if s.window_title.trim().is_empty() {
        "TampilHP - Stream".to_string()
    } else {
        s.window_title.trim().to_string()
    };
    args.push(format!("--window-title={title}"));

    if s.max_size > 0 {
        args.push(format!("--max-size={}", s.max_size));
    }
    if s.max_fps > 0 {
        args.push(format!("--max-fps={}", s.max_fps));
    }
    if s.bitrate_mbps > 0 {
        args.push(format!("--video-bit-rate={}M", s.bitrate_mbps));
    }
    if !s.video_codec.is_empty() {
        args.push(format!("--video-codec={}", s.video_codec));
    }
    if s.render_driver != "auto" && !s.render_driver.is_empty() {
        args.push(format!("--render-driver={}", s.render_driver));
    }
    if s.video_buffer_ms > 0 {
        args.push(format!("--video-buffer={}", s.video_buffer_ms));
    }

    match s.audio_mode.as_str() {
        "none" => args.push("--no-audio".into()),
        "playback" => {
            args.push("--audio-source=playback".into());
            args.push(format!("--audio-buffer={}", s.audio_buffer_ms));
        }
        _ => {
            args.push("--audio-source=output".into());
            args.push(format!("--audio-buffer={}", s.audio_buffer_ms));
        }
    }

    if s.borderless {
        args.push("--window-borderless".into());
    }
    if s.always_on_top {
        args.push("--always-on-top".into());
    }
    if s.turn_screen_off {
        args.push("--turn-screen-off".into());
    }
    if s.stay_awake {
        args.push("--stay-awake".into());
    }
    if s.no_control {
        args.push("--no-control".into());
    }
    if !s.record_path.trim().is_empty() {
        args.push(format!("--record={}", s.record_path.trim()));
    }

    args
}

pub async fn start_mirror(
    app: AppHandle,
    manager: &ScrcpyManager,
    serial: String,
    settings: MirrorSettings,
) -> Result<Vec<String>, String> {
    // Stop existing mirror for this serial if running
    let _ = stop_mirror(manager, &serial).await;

    let scrcpy_exe = scrcpy_path(&app)?;
    let adb_exe = adb_path(&app)?;
    let scrcpy_dir = resolve_scrcpy_dir(&app)?;
    let args = build_scrcpy_args(&serial, &settings);

    let mut cmd = tokio::process::Command::new(&scrcpy_exe);
    cmd.args(&args)
        .current_dir(&scrcpy_dir)
        .env("ADB", &adb_exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        cmd.creation_flags(crate::paths::CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menjalankan scrcpy: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    {
        let mut guard = manager.sessions.lock().await;
        guard.insert(serial.clone(), child);
    }

    // Stream stdout
    if let Some(out) = stdout {
        let app_c = app.clone();
        let serial_c = serial.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_c.emit(
                    "scrcpy-log",
                    ScrcpyLogEvent {
                        serial: serial_c.clone(),
                        level: "info".into(),
                        line,
                    },
                );
            }
        });
    }

    // Stream stderr and capture hint
    let hint_holder = Arc::new(Mutex::new(None::<String>));
    if let Some(err) = stderr {
        let app_c = app.clone();
        let serial_c = serial.clone();
        let hint_c = hint_holder.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(h) = classify_error_hint(&line) {
                    *hint_c.lock().await = Some(h);
                }
                let _ = app_c.emit(
                    "scrcpy-log",
                    ScrcpyLogEvent {
                        serial: serial_c.clone(),
                        level: "error".into(),
                        line,
                    },
                );
            }
        });
    }

    // Watch exit
    let sessions_c = manager.sessions.clone();
    let app_c = app.clone();
    let serial_c = serial.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let mut guard = sessions_c.lock().await;
            let finished = match guard.get_mut(&serial_c) {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => Some(status.code()),
                    Ok(None) => None,
                    Err(_) => Some(None),
                },
                None => break,
            };

            if let Some(code) = finished {
                guard.remove(&serial_c);
                drop(guard);
                let hint = hint_holder.lock().await.clone();
                let _ = app_c.emit(
                    "scrcpy-exit",
                    ScrcpyExitEvent {
                        serial: serial_c,
                        code,
                        hint,
                    },
                );
                break;
            }
        }
    });

    Ok(args)
}

pub async fn stop_mirror(manager: &ScrcpyManager, serial: &str) -> Result<(), String> {
    let mut guard = manager.sessions.lock().await;
    if let Some(mut child) = guard.remove(serial) {
        let _ = child.kill().await;
    }
    Ok(())
}

pub async fn stop_all_mirrors(manager: &ScrcpyManager) {
    let mut guard = manager.sessions.lock().await;
    for (_, mut child) in guard.drain() {
        let _ = child.kill().await;
    }
}

pub async fn active_serials(manager: &ScrcpyManager) -> Vec<String> {
    let guard = manager.sessions.lock().await;
    guard.keys().cloned().collect()
}

fn classify_error_hint(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if lower.contains("inject_events") || lower.contains("securityexception") {
        return Some(
            "HP Xiaomi/HyperOS/Oppo butuh aktifkan 'USB debugging (Security settings)' di Developer Options agar sentuhan & mouse bekerja."
                .into(),
        );
    }
    if lower.contains("unauthorized") {
        return Some("Cek layar HP, centang 'Always allow', lalu tekan Allow.".into());
    }
    if lower.contains("audio") && (lower.contains("failed") || lower.contains("unsupported")) {
        return Some(
            "Audio forwarding gagal (butuh Android 11+). Coba ubah Audio Mode ke 'Tanpa Audio'."
                .into(),
        );
    }
    if lower.contains("renderer") || lower.contains("createwindow") {
        return Some(
            "Gagal inisialisasi renderer. Coba ganti Render Driver ke Direct3D atau OpenGL."
                .into(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tiktok_preset_args_with_serial() {
        let mut s = MirrorSettings::default();
        s.render_driver = "opengl".into();
        s.turn_screen_off = true;
        s.window_title = "TampilHP - TikTok Live".into();

        let args = build_scrcpy_args("192.168.1.20:5555", &s);
        assert_eq!(args[0], "-s");
        assert_eq!(args[1], "192.168.1.20:5555");
        assert!(args.contains(&"--render-driver=opengl".to_string()));
        assert!(args.contains(&"--turn-screen-off".to_string()));
        assert!(args.contains(&"--window-title=TampilHP - TikTok Live".to_string()));
    }

    #[test]
    fn classifies_miui_security_error() {
        let hint = classify_error_hint("java.lang.SecurityException: Injecting input events requires INJECT_EVENTS");
        assert!(hint.is_some());
    }
}
