use crate::adb::list_devices;
use crate::paths::background_cmd;
use serde::Serialize;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticItem {
    pub id: String,
    pub title: String,
    pub status: String, // "ok" | "warn" | "error"
    pub detail: String,
    pub fix_action: Option<String>,
}

pub async fn run_diagnostics(adb: &PathBuf, scrcpy: &PathBuf, test_ip: Option<String>) -> Vec<DiagnosticItem> {
    let mut items = Vec::new();

    // 1. Bundled binary check
    let bin_ok = adb.exists() && scrcpy.exists();
    items.push(DiagnosticItem {
        id: "binaries".into(),
        title: "Engine scrcpy & ADB Bawaan".into(),
        status: if bin_ok { "ok" } else { "error" }.into(),
        detail: if bin_ok {
            format!("Siap ({})", scrcpy.display())
        } else {
            "File scrcpy.exe atau adb.exe tidak ditemukan di folder resources.".into()
        },
        fix_action: None,
    });

    // 2. Device state check
    match list_devices(adb).await {
        Ok(devs) => {
            let unauthorized = devs.iter().filter(|d| d.state == "unauthorized").count();
            let offline = devs.iter().filter(|d| d.state == "offline").count();
            let ready = devs.iter().filter(|d| d.state == "device").count();

            if unauthorized > 0 {
                items.push(DiagnosticItem {
                    id: "trust".into(),
                    title: "Otorisasi USB Debugging".into(),
                    status: "error".into(),
                    detail: format!(
                        "{unauthorized} HP status 'unauthorized'. Buka layar HP, centang 'Always allow from this computer', lalu tekan Allow."
                    ),
                    fix_action: None,
                });
            }

            if offline > 0 {
                items.push(DiagnosticItem {
                    id: "offline".into(),
                    title: "Ghost / Offline Device".into(),
                    status: "warn".into(),
                    detail: format!(
                        "{offline} koneksi wireless menyangkut status 'offline' dan sering bikin gagal connect ulang."
                    ),
                    fix_action: Some("clean_offline".into()),
                });
            }

            if ready > 0 {
                items.push(DiagnosticItem {
                    id: "ready".into(),
                    title: "Perangkat Terhubung".into(),
                    status: "ok".into(),
                    detail: format!("{ready} perangkat siap di-mirror (target otomatis via -s serial)."),
                    fix_action: None,
                });
            } else if unauthorized == 0 && offline == 0 {
                items.push(DiagnosticItem {
                    id: "nodev".into(),
                    title: "Belum Ada HP Terdeteksi".into(),
                    status: "warn".into(),
                    detail: "Colok kabel USB (USB Debugging aktif) atau gunakan Tab Wireless untuk pair tanpa kabel.".into(),
                    fix_action: None,
                });
            }
        }
        Err(e) => {
            items.push(DiagnosticItem {
                id: "adb_daemon".into(),
                title: "ADB Daemon Bermasalah".into(),
                status: "error".into(),
                detail: format!("Gagal membaca perangkat: {e}. Kemungkinan bentrok dengan adb.exe lain di PC."),
                fix_action: Some("restart_adb".into()),
            });
        }
    }

    // 3. VPN / multiple network check on Windows via ipconfig
    let vpn_active = check_vpn_adapter().await;
    if vpn_active {
        items.push(DiagnosticItem {
            id: "vpn".into(),
            title: "VPN Terdeteksi Aktif".into(),
            status: "warn".into(),
            detail: "Adapter VPN aktif dapat memblokir koneksi lokal Wi-Fi antara PC dan HP. Matikan VPN jika wireless gagal.".into(),
            fix_action: None,
        });
    }

    // 4. Optional TCP port test on target IP:5555
    if let Some(ip_raw) = test_ip {
        let trimmed = ip_raw.trim().to_string();
        if !trimmed.is_empty() {
            let addr = if trimmed.contains(':') {
                trimmed.clone()
            } else {
                format!("{trimmed}:5555")
            };
            let addr_clone = addr.clone();
            let reachable = tokio::task::spawn_blocking(move || {
                if let Ok(sock) = addr_clone.parse() {
                    TcpStream::connect_timeout(&sock, Duration::from_millis(1200)).is_ok()
                } else {
                    false
                }
            })
            .await
            .unwrap_or(false);

            items.push(DiagnosticItem {
                id: "port_test".into(),
                title: format!("Tes Koneksi {addr}"),
                status: if reachable { "ok" } else { "error" }.into(),
                detail: if reachable {
                    "Port terbuka dan bisa dijangkau dari PC.".into()
                } else {
                    "Port tidak merespons. Cek: (1) HP & PC satu Wi-Fi, (2) Sudah aktifkan TCP/IP 5555 via USB / Wireless Debugging, (3) Matikan AP Isolation di router & cek Windows Firewall.".into()
                },
                fix_action: None,
            });
        }
    }

    items
}

async fn check_vpn_adapter() -> bool {
    if let Ok(out) = background_cmd(&PathBuf::from("ipconfig")).output().await {
        let txt = String::from_utf8_lossy(&out.stdout).to_lowercase();
        return txt.contains("wireguard")
            || txt.contains("openvpn")
            || txt.contains("wintun")
            || txt.contains("nordlynx")
            || txt.contains("tailscale");
    }
    false
}
