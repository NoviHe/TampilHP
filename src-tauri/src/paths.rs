use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Creates an async tokio Command with CREATE_NO_WINDOW on Windows so background
/// adb / diagnostics calls never flash a console window.
pub fn background_cmd(program: &PathBuf) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Resolves the directory containing bundled scrcpy.exe, scrcpy-server, adb.exe, and DLLs.
pub fn resolve_scrcpy_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("resources").join("scrcpy"));
            candidates.push(exe_dir.join("scrcpy"));
        }
    }

    if let Ok(res_dir) = app.path().resource_dir() {
        candidates.push(res_dir.join("resources").join("scrcpy"));
        candidates.push(res_dir.join("scrcpy"));
    }

    // Fallback for `cargo tauri dev`
    let manifest_scrcpy = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("scrcpy");
    candidates.push(manifest_scrcpy);

    for dir in &candidates {
        if dir.join("scrcpy.exe").exists() && dir.join("adb.exe").exists() {
            return Ok(dir.clone());
        }
    }

    Err(format!(
        "Bundled scrcpy.exe / adb.exe tidak ditemukan. Checked: {:?}",
        candidates
    ))
}

pub fn adb_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_scrcpy_dir(app)?.join("adb.exe"))
}

pub fn scrcpy_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_scrcpy_dir(app)?.join("scrcpy.exe"))
}

/// Returns the exe directory if running in portable mode (`portable.flag` exists
/// or `resources/scrcpy` is colocated next to the executable in a writable folder).
pub fn resolve_portable_dir() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;
    resolve_portable_dir_from(exe_dir)
}

fn resolve_portable_dir_from(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.join("portable.flag").exists() {
        return Some(exe_dir.to_path_buf());
    }
    let has_local_scrcpy = exe_dir.join("resources").join("scrcpy").join("scrcpy.exe").exists();
    if has_local_scrcpy && is_dir_writable(exe_dir) {
        return Some(exe_dir.to_path_buf());
    }
    None
}

fn is_dir_writable(dir: &Path) -> bool {
    let probe = dir.join(".tampilhp_write_test");
    if std::fs::write(&probe, b"").is_ok() {
        let _ = std::fs::remove_file(&probe);
        true
    } else {
        false
    }
}

pub fn app_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(portable_dir) = resolve_portable_dir() {
        return Ok(portable_dir.join("config.json"));
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Gagal akses app_data_dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Gagal buat folder config: {e}"))?;
    Ok(dir.join("config.json"))
}

pub fn screenshot_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let out_dir = if let Some(portable_dir) = resolve_portable_dir() {
        portable_dir.join("screenshots")
    } else {
        let pics_dir = app
            .path()
            .picture_dir()
            .or_else(|_| app.path().desktop_dir())
            .map_err(|e| format!("Gagal akses folder Pictures: {e}"))?;
        pics_dir.join("TampilHP")
    };
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("Gagal buat folder screenshot: {e}"))?;
    Ok(out_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_portable_flag_dir() {
        let tmp = std::env::temp_dir().join(format!("tampilhp_portable_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(resolve_portable_dir_from(&tmp).is_none());

        std::fs::write(tmp.join("portable.flag"), b"portable").unwrap();
        assert_eq!(resolve_portable_dir_from(&tmp), Some(tmp.clone()));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
