# TampilHP

Aplikasi desktop Windows yang ringan dan *low-latency* untuk screen mirroring Android berbasis **scrcpy (v5.0.1)** dan **Tauri v2**, dirancang dengan menu klik praktis untuk kebutuhan harian, gaming, dan livestreaming (**OBS Studio**, **Streamlabs**, **TikTok LIVE Studio**).

## Fitur Utama

- **1-Klik USB-to-WiFi (Android 5+)**: Colok kabel USB, klik **1-Klik Wi-Fi**, lalu cabut kabel setelah notifikasi muncul. Aplikasi otomatis membaca IP `wlan0`, mengaktifkan `adb tcpip 5555`, dan memverifikasi koneksi.
- **Tanpa Kabel Total (Android 11+)**:
  - **QR Code Pairing**: Generate QR langsung di aplikasi dan scan dari *Developer Options → Wireless Debugging → Pair device with QR code* (auto-pair & auto-connect via mDNS).
  - **Pairing Code**: Masukkan `IP:Port` + kode 6-digit dengan auto-discovery port koneksi via mDNS.
  - **Scan Jaringan (mDNS)** & koneksi manual `IP:Port`.
- **Preset Siap Pakai**:
  - **Gaming Ultra Low-Latency**: H.264, `1080p`, `60 FPS`, `16 Mbps`, `--video-buffer=0`, `--audio-buffer=20`.
  - **Wireless Stabil**: `1280p`, `60 FPS`, `8 Mbps`, layar HP otomatis mati (`--turn-screen-off`).
  - **OBS / Streamlabs**: Judul window tetap (`TampilHP - OBS Stream`) + renderer `Direct3D`.
  - **TikTok LIVE Studio**: Renderer `OpenGL` (mengatasi *black screen* pada Window Capture) + judul window tetap (`TampilHP - TikTok Live`).
- **Kontrol Cepat & Streamer Tools**:
  - Switcher **Render Driver** (`Direct3D`, `OpenGL`, `Software`, `Auto`).
  - **Audio Mode**: Forward audio ke PC (`output`), `playback`, atau tanpa audio (`--no-audio`).
  - Toggle *Matikan Layar HP* (tetap bisa dikontrol full dari PC), *Stay Awake*, *Window Borderless*, *Always on Top*, dan *View Only*.
  - Screenshot langsung ke folder lokal dan System Tray icon.
- **Diagnostik Koneksi & Anti-Bentrok ADB**:
  - Menggunakan `adb.exe` dan `scrcpy.exe` terisolasi agar tidak bentrok dengan ADB lain di sistem.
  - Selalu menjalankan target spesifik `-s <serial>` sehingga aman saat beberapa HP/emulator terhubung.
  - Pembersihan otomatis perangkat `offline`, deteksi status `unauthorized`, adapter VPN, dan tes port TCP target.
- **True Portable**: Mendukung penyimpanan `config.json` dan `screenshots/` langsung di folder aplikasi saat `portable.flag` tersedia.

## Prasyarat Development (Windows)

- [Node.js](https://nodejs.org/) (v18+)
- [Rust & Cargo](https://rustup.rs/)
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (sudah terpasang di Windows 11 dan mayoritas Windows 10)

## Cara Menjalankan (Development)

```powershell
# 1. Install dependencies frontend
npm install

# 2. Unduh binary resmi scrcpy v5.0.1 + adb ke src-tauri/resources/scrcpy
powershell -ExecutionPolicy Bypass -File scripts/fetch-scrcpy.ps1

# 3. Jalankan mode development
npm run tauri dev
```

## Build True Portable (`.exe` + `.zip`)

```powershell
npm run build:portable
```

Hasil build akan tersimpan di:
- `dist-portable/TampilHP-Portable/` (`tampilhp.exe`, `portable.flag`, `resources/scrcpy/`)
- `dist-portable/TampilHP-Portable-win64.zip`

## Tips OBS / Streamlabs / TikTok LIVE Studio

1. Gunakan **Window Capture** dan pilih window dengan judul sesuai pengaturan (contoh: `TampilHP - OBS Stream` atau `TampilHP - TikTok Live`).
2. Jika tampilan capture hitam (*black screen*), ubah **Render Driver** ke **OpenGL** atau **Software**, dan pastikan window mirror tidak di-minimize.
3. Untuk menangkap suara HP terpisah dari suara desktop lain, gunakan **Application Audio Capture** dan arahkan ke `scrcpy.exe`.

## Troubleshooting Singkat

- **Status `unauthorized` di daftar HP**: Buka layar HP, centang **Always allow from this computer**, lalu tekan **Allow**.
- **Layar tampil tapi tidak bisa diklik (Xiaomi / HyperOS / Oppo / Vivo)**: Aktifkan **USB debugging (Security settings)** di *Developer Options* HP.
- **Wireless gagal connect**: Pastikan HP dan PC di jaringan Wi-Fi yang sama, matikan VPN sementara, lalu klik **Diagnostik** atau **Bersihkan Offline** di TampilHP.

## Lisensi Engine

Menggunakan [Genymobile/scrcpy](https://github.com/Genymobile/scrcpy) (Apache-2.0) sebagai engine mirroring.
