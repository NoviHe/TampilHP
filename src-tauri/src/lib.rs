mod adb;
mod diagnostics;
mod mdns;
mod paths;
mod presets;
mod scrcpy;

use adb::{
    clean_offline_devices, connect_wireless, disconnect_device, enable_wireless_from_usb,
    list_devices, pair_device, restart_adb_server, AdbDevice, WirelessSwitchResult,
};
use diagnostics::{run_diagnostics, DiagnosticItem};
use mdns::{create_qr_session, discover_tls_connect_services, wait_and_pair_qr, DiscoveredAdbService, QrSessionInfo};
use paths::{adb_path, scrcpy_path};
use presets::{builtin_presets, load_config, save_config, AppConfig, MirrorSettings, PresetDefinition};
use scrcpy::{active_serials, start_mirror, stop_all_mirrors, stop_mirror, ScrcpyManager};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

#[tauri::command]
async fn cmd_list_devices(app: AppHandle) -> Result<Vec<AdbDevice>, String> {
    let adb = adb_path(&app)?;
    list_devices(&adb).await
}

#[tauri::command]
async fn cmd_enable_wireless_usb(app: AppHandle, serial: String) -> Result<WirelessSwitchResult, String> {
    let adb = adb_path(&app)?;
    enable_wireless_from_usb(&adb, &serial).await
}

#[tauri::command]
async fn cmd_connect_wireless(app: AppHandle, address: String) -> Result<String, String> {
    let adb = adb_path(&app)?;
    connect_wireless(&adb, &address).await
}

#[tauri::command]
async fn cmd_pair_and_connect(
    app: AppHandle,
    ip: String,
    pair_port: u16,
    code: String,
    connect_port: Option<u16>,
) -> Result<String, String> {
    let adb = adb_path(&app)?;
    pair_device(&adb, &ip, pair_port, &code).await?;

    if let Some(cport) = connect_port.filter(|p| *p > 0) {
        let target = format!("{}:{}", ip.trim(), cport);
        connect_wireless(&adb, &target).await?;
        return Ok(target);
    }

    // Auto-discover connect port via mDNS
    for _ in 0..4 {
        if let Ok(services) = discover_tls_connect_services(1200).await {
            if let Some(svc) = services.into_iter().find(|s| s.ip == ip.trim()) {
                let target = format!("{}:{}", svc.ip, svc.port);
                connect_wireless(&adb, &target).await?;
                return Ok(target);
            }
        }
    }

    Err("Pairing sukses, tetapi port koneksi otomatis tidak ditemukan. Masukkan IP:Port koneksi di kolom Connect Manual.".into())
}

#[tauri::command]
async fn cmd_start_qr_pairing(app: AppHandle) -> Result<QrSessionInfo, String> {
    let adb = adb_path(&app)?;
    let session = create_qr_session();
    let s_name = session.service_name.clone();
    let pass = session.password.clone();
    tokio::spawn(async move {
        wait_and_pair_qr(app, adb, s_name, pass).await;
    });
    Ok(session)
}

#[tauri::command]
async fn cmd_discover_wireless() -> Result<Vec<DiscoveredAdbService>, String> {
    discover_tls_connect_services(1800).await
}

#[tauri::command]
async fn cmd_disconnect_device(app: AppHandle, serial: String) -> Result<String, String> {
    let adb = adb_path(&app)?;
    disconnect_device(&adb, &serial).await
}

#[tauri::command]
async fn cmd_clean_offline(app: AppHandle) -> Result<usize, String> {
    let adb = adb_path(&app)?;
    clean_offline_devices(&adb).await
}

#[tauri::command]
async fn cmd_restart_adb(app: AppHandle) -> Result<String, String> {
    let adb = adb_path(&app)?;
    restart_adb_server(&adb).await
}

#[tauri::command]
async fn cmd_start_mirror(
    app: AppHandle,
    state: State<'_, ScrcpyManager>,
    serial: String,
    settings: MirrorSettings,
) -> Result<Vec<String>, String> {
    start_mirror(app, &state, serial, settings).await
}

#[tauri::command]
async fn cmd_stop_mirror(state: State<'_, ScrcpyManager>, serial: String) -> Result<(), String> {
    stop_mirror(&state, &serial).await
}

#[tauri::command]
async fn cmd_active_mirrors(state: State<'_, ScrcpyManager>) -> Result<Vec<String>, String> {
    Ok(active_serials(&state).await)
}

#[tauri::command]
fn cmd_get_presets() -> Vec<PresetDefinition> {
    builtin_presets()
}

#[tauri::command]
fn cmd_load_config(app: AppHandle) -> AppConfig {
    load_config(&app)
}

#[tauri::command]
fn cmd_save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    save_config(&app, &config)
}

#[tauri::command]
async fn cmd_run_diagnostics(
    app: AppHandle,
    test_ip: Option<String>,
) -> Result<Vec<DiagnosticItem>, String> {
    let adb = adb_path(&app)?;
    let scrcpy = scrcpy_path(&app)?;
    Ok(run_diagnostics(&adb, &scrcpy, test_ip).await)
}

#[tauri::command]
async fn cmd_take_screenshot(app: AppHandle, serial: String) -> Result<String, String> {
    let adb = adb_path(&app)?;
    let out_dir = paths::screenshot_output_dir(&app)?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let file_path = out_dir.join(format!("screenshot-{ts}.png"));

    let out = paths::background_cmd(&adb)
        .args(["-s", &serial, "exec-out", "screencap", "-p"])
        .output()
        .await
        .map_err(|e| format!("Gagal screenshot: {e}"))?;

    if !out.status.success() || out.stdout.is_empty() {
        return Err("Screenshot gagal diambil dari HP.".into());
    }
    std::fs::write(&file_path, &out.stdout)
        .map_err(|e| format!("Gagal simpan screenshot: {e}"))?;

    Ok(file_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScrcpyManager::default())
        .setup(|app| {
            let show_i = MenuItem::with_id(app, "show", "Buka TampilHP", true, None::<&str>)?;
            let stop_i = MenuItem::with_id(app, "stop_all", "Stop Semua Mirror", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &stop_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("TampilHP — Android Mirror & Stream");
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            tray_builder
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "stop_all" => {
                        let app_c = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let mgr = app_c.state::<ScrcpyManager>();
                            stop_all_mirrors(&mgr).await;
                        });
                    }
                    "quit" => {
                        let app_c = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let mgr = app_c.state::<ScrcpyManager>();
                            stop_all_mirrors(&mgr).await;
                            app_c.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_list_devices,
            cmd_enable_wireless_usb,
            cmd_connect_wireless,
            cmd_pair_and_connect,
            cmd_start_qr_pairing,
            cmd_discover_wireless,
            cmd_disconnect_device,
            cmd_clean_offline,
            cmd_restart_adb,
            cmd_start_mirror,
            cmd_stop_mirror,
            cmd_active_mirrors,
            cmd_get_presets,
            cmd_load_config,
            cmd_save_config,
            cmd_run_diagnostics,
            cmd_take_screenshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
