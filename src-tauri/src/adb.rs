use crate::paths::background_cmd;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub transport: String,
    pub model: String,
    pub product: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessSwitchResult {
    pub target_serial: String,
    pub ip: String,
    pub message: String,
}

async fn run_adb(adb: &PathBuf, args: &[&str]) -> Result<String, String> {
    let out = background_cmd(adb)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Gagal menjalankan adb: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if out.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        let msg = if stderr.is_empty() { stdout } else { stderr };
        Err(msg)
    }
}

pub async fn list_devices(adb: &PathBuf) -> Result<Vec<AdbDevice>, String> {
    let raw = run_adb(adb, &["devices", "-l"]).await?;
    let mut devices = Vec::new();

    for line in raw.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let serial = match parts.next() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let state = match parts.next() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let mut model = String::new();
        let mut product = String::new();
        for token in parts {
            if let Some(v) = token.strip_prefix("model:") {
                model = v.replace('_', " ");
            } else if let Some(v) = token.strip_prefix("product:") {
                product = v.to_string();
            }
        }

        let is_wifi = serial.contains(':') || serial.contains("_adb-tls-connect");
        let transport = if is_wifi { "wifi" } else { "usb" }.to_string();
        let ip = if serial.contains(':') {
            serial.split(':').next().map(|s| s.to_string())
        } else {
            None
        };

        if model.is_empty() {
            model = if is_wifi {
                "Android Wireless".to_string()
            } else {
                "Android USB".to_string()
            };
        }

        devices.push(AdbDevice {
            serial,
            state,
            transport,
            model,
            product,
            ip,
        });
    }

    Ok(devices)
}

pub async fn get_device_wifi_ip(adb: &PathBuf, serial: &str) -> Result<String, String> {
    // Primary: inspect wlan0 inet address
    if let Ok(out) = run_adb(adb, &["-s", serial, "shell", "ip", "-f", "inet", "addr", "show", "wlan0"]).await {
        if let Some(ip) = extract_inet_ip(&out) {
            return Ok(ip);
        }
    }

    // Fallback: inspect routing table (`src <ip>`)
    if let Ok(out) = run_adb(adb, &["-s", serial, "shell", "ip", "route"]).await {
        for line in out.lines() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            for pair in tokens.windows(2) {
                if pair[0] == "src" && is_ipv4(pair[1]) && !pair[1].starts_with("127.") {
                    return Ok(pair[1].to_string());
                }
            }
        }
    }

    Err("IP Wi-Fi HP tidak ditemukan. Pastikan HP sudah konek ke Wi-Fi yang sama dengan PC.".to_string())
}

fn extract_inet_ip(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            if let Some(ip_cidr) = rest.split_whitespace().next() {
                if let Some(ip) = ip_cidr.split('/').next() {
                    if is_ipv4(ip) && !ip.starts_with("127.") {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }
    None
}

fn is_ipv4(s: &str) -> bool {
    s.parse::<std::net::Ipv4Addr>().is_ok()
}

pub async fn enable_wireless_from_usb(
    adb: &PathBuf,
    usb_serial: &str,
) -> Result<WirelessSwitchResult, String> {
    let ip = get_device_wifi_ip(adb, usb_serial).await?;

    // Clean stale offline entries first so adb won't get stuck on ghost sockets
    let _ = clean_offline_devices(adb).await;

    run_adb(adb, &["-s", usb_serial, "tcpip", "5555"])
        .await
        .map_err(|e| format!("Gagal mengaktifkan mode TCP/IP di HP: {e}"))?;

    let target = format!("{ip}:5555");
    let mut last_err = String::new();

    for _ in 0..8 {
        sleep(Duration::from_millis(650)).await;
        match run_adb(adb, &["connect", &target]).await {
            Ok(msg) if msg.contains("connected") || msg.contains("already connected") => {
                // Verify it actually shows up as `device` (not offline)
                if let Ok(devs) = list_devices(adb).await {
                    if devs.iter().any(|d| d.serial == target && d.state == "device") {
                        return Ok(WirelessSwitchResult {
                            target_serial: target,
                            ip,
                            message: "Wireless aktif! Anda bisa mencabut kabel USB sekarang.".to_string(),
                        });
                    }
                }
                last_err = msg;
            }
            Ok(msg) => last_err = msg,
            Err(e) => last_err = e,
        }
    }

    Err(format!(
        "Port 5555 aktif di {ip}, tetapi PC gagal konek ({last_err}). Cek Wi-Fi PC/HP satu jaringan & tidak diblokir firewall/VPN."
    ))
}

pub async fn connect_wireless(adb: &PathBuf, address: &str) -> Result<String, String> {
    let normalized = if address.contains(':') {
        address.trim().to_string()
    } else {
        format!("{}:5555", address.trim())
    };
    let _ = clean_offline_devices(adb).await;
    let out = run_adb(adb, &["connect", &normalized]).await?;
    if out.contains("unable") || out.contains("failed") || out.contains("cannot") {
        return Err(format!("Gagal konek ke {normalized}: {out}"));
    }
    Ok(out)
}

pub async fn pair_device(
    adb: &PathBuf,
    ip: &str,
    pair_port: u16,
    code: &str,
) -> Result<String, String> {
    let addr = format!("{}:{}", ip.trim(), pair_port);
    let out = run_adb(adb, &["pair", &addr, code.trim()]).await?;
    if out.to_lowercase().contains("failed") {
        return Err(format!("Pairing gagal: {out}"));
    }
    Ok(out)
}

pub async fn disconnect_device(adb: &PathBuf, serial: &str) -> Result<String, String> {
    run_adb(adb, &["disconnect", serial]).await
}

pub async fn clean_offline_devices(adb: &PathBuf) -> Result<usize, String> {
    let devices = list_devices(adb).await?;
    let mut cleaned = 0usize;
    for dev in devices {
        if dev.state == "offline" && dev.transport == "wifi" {
            if run_adb(adb, &["disconnect", &dev.serial]).await.is_ok() {
                cleaned += 1;
            }
        }
    }
    Ok(cleaned)
}

pub async fn restart_adb_server(adb: &PathBuf) -> Result<String, String> {
    let _ = run_adb(adb, &["kill-server"]).await;
    #[cfg(windows)]
    {
        // Kill any stray adb.exe from C:\platform-tools or old scrcpy folders
        let mut kill_cmd = tokio::process::Command::new("taskkill");
        kill_cmd.args(["/F", "/IM", "adb.exe"]);
        kill_cmd.creation_flags(crate::paths::CREATE_NO_WINDOW);
        let _ = kill_cmd.output().await;
    }
    sleep(Duration::from_millis(300)).await;
    run_adb(adb, &["start-server"]).await?;
    Ok("ADB server berhasil di-restart menggunakan binary bawaan TampilHP.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wlan0_inet_ip() {
        let sample = "32: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500\n    inet 192.168.1.42/24 brd 192.168.1.255 scope global wlan0\n";
        assert_eq!(extract_inet_ip(sample), Some("192.168.1.42".to_string()));
    }

    #[test]
    fn ignores_loopback_ip() {
        let sample = "    inet 127.0.0.1/8 scope host lo\n";
        assert_eq!(extract_inet_ip(sample), None);
    }
}
