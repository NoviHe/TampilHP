use crate::adb::{connect_wireless, pair_device};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrSessionInfo {
    pub service_name: String,
    pub password: String,
    pub qr_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAdbService {
    pub instance_name: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct QrPairEvent {
    pub status: String,
    pub message: String,
    pub target_serial: Option<String>,
}

pub fn create_qr_session() -> QrSessionInfo {
    let mut rng = rand::thread_rng();
    let suffix: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let password: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let service_name = format!("studio-{suffix}");
    let qr_payload = format!("WIFI:T:ADB;S:{service_name};P:{password};;");
    QrSessionInfo {
        service_name,
        password,
        qr_payload,
    }
}

/// Discovers devices currently advertising `_adb-tls-connect._tcp.local.` on the LAN.
pub async fn discover_tls_connect_services(timeout_ms: u64) -> Result<Vec<DiscoveredAdbService>, String> {
    tokio::task::spawn_blocking(move || {
        let mdns = ServiceDaemon::new().map_err(|e| format!("Gagal start mDNS: {e}"))?;
        let receiver = mdns
            .browse("_adb-tls-connect._tcp.local.")
            .map_err(|e| format!("Gagal browse mDNS: {e}"))?;

        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        let mut results: Vec<DiscoveredAdbService> = Vec::new();

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            match receiver.recv_timeout(remaining.min(Duration::from_millis(250))) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    if let Some(addr) = info.get_addresses_v4().into_iter().next() {
                        let ip = addr.to_string();
                        let port = info.get_port();
                        if !results.iter().any(|r| r.ip == ip && r.port == port) {
                            results.push(DiscoveredAdbService {
                                instance_name: info.get_fullname().to_string(),
                                ip,
                                port,
                            });
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }

        let _ = mdns.shutdown();
        Ok(results)
    })
    .await
    .map_err(|e| format!("mDNS task error: {e}"))?
}

/// Waits for Android 11+ QR pairing mDNS broadcast matching `service_name`,
/// runs `adb pair`, then resolves `_adb-tls-connect._tcp.local.` to auto-connect.
pub async fn wait_and_pair_qr(
    app: AppHandle,
    adb: PathBuf,
    service_name: String,
    password: String,
) {
    let _ = app.emit(
        "qr-pair-status",
        QrPairEvent {
            status: "waiting".to_string(),
            message: "Scan QR Code dari HP: Opsi Pengembang -> Wireless Debugging -> Pair device with QR code".to_string(),
            target_serial: None,
        },
    );

    let service_match = service_name.clone();
    let paired_target = tokio::task::spawn_blocking(move || -> Result<(String, u16), String> {
        let mdns = ServiceDaemon::new().map_err(|e| format!("Gagal start mDNS: {e}"))?;
        let receiver = mdns
            .browse("_adb-tls-pairing._tcp.local.")
            .map_err(|e| format!("Gagal browse mDNS pairing: {e}"))?;

        let deadline = std::time::Instant::now() + Duration::from_secs(90);
        let mut found: Option<(String, u16)> = None;

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            match receiver.recv_timeout(remaining.min(Duration::from_millis(300))) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    if info.get_fullname().contains(&service_match) {
                        if let Some(ip) = info.get_addresses_v4().into_iter().next() {
                            found = Some((ip.to_string(), info.get_port()));
                            break;
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }

        let _ = mdns.shutdown();
        found.ok_or_else(|| "Timeout 90 detik menunggu scan QR dari HP.".to_string())
    })
    .await;

    let (ip, pair_port) = match paired_target {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => {
            let _ = app.emit(
                "qr-pair-status",
                QrPairEvent {
                    status: "error".to_string(),
                    message: e,
                    target_serial: None,
                },
            );
            return;
        }
        Err(e) => {
            let _ = app.emit(
                "qr-pair-status",
                QrPairEvent {
                    status: "error".to_string(),
                    message: format!("mDNS error: {e}"),
                    target_serial: None,
                },
            );
            return;
        }
    };

    if let Err(e) = pair_device(&adb, &ip, pair_port, &password).await {
        let _ = app.emit(
            "qr-pair-status",
            QrPairEvent {
                status: "error".to_string(),
                message: e,
                target_serial: None,
            },
        );
        return;
    }

    let _ = app.emit(
        "qr-pair-status",
        QrPairEvent {
            status: "paired".to_string(),
            message: format!("Pairing sukses ({ip}:{pair_port}). Mencari port koneksi otomatis..."),
            target_serial: None,
        },
    );

    // Resolve _adb-tls-connect for this IP (up to 6s)
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if let Ok(services) = discover_tls_connect_services(1200).await {
            if let Some(svc) = services.into_iter().find(|s| s.ip == ip) {
                let target = format!("{}:{}", svc.ip, svc.port);
                match connect_wireless(&adb, &target).await {
                    Ok(_) => {
                        let _ = app.emit(
                            "qr-pair-status",
                            QrPairEvent {
                                status: "connected".to_string(),
                                message: format!("Terhubung wireless ke {target}!"),
                                target_serial: Some(target),
                            },
                        );
                        return;
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "qr-pair-status",
                            QrPairEvent {
                                status: "error".to_string(),
                                message: e,
                                target_serial: None,
                            },
                        );
                        return;
                    }
                }
            }
        }
    }

    let _ = app.emit(
        "qr-pair-status",
        QrPairEvent {
            status: "paired".to_string(),
            message: format!("Pairing berhasil! Klik connect pada {ip} di daftar Wireless Discovery atau masukkan IP:Port di Wireless Debugging."),
            target_serial: None,
        },
    );
}
