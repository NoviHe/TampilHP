import "./styles/app.css";
import { listen } from "@tauri-apps/api/event";
import QRCode from "qrcode";
import { api } from "./api";
import type {
  AdbDevice,
  AppConfig,
  DiagnosticItem,
  DiscoveredAdbService,
  MirrorSettings,
  PresetDefinition,
  QrPairEvent,
  QrSessionInfo,
  ScrcpyExitEvent,
  ScrcpyLogEvent,
} from "./types";

interface AppState {
  devices: AdbDevice[];
  activeSerials: string[];
  presets: PresetDefinition[];
  config: AppConfig;
  wirelessTab: "qr" | "code" | "manual";
  qrSession: QrSessionInfo | null;
  qrStatus: string;
  discovered: DiscoveredAdbService[];
  diagnostics: DiagnosticItem[];
  showDiagnostics: boolean;
  logs: string[];
  busyAction: string | null;
  toast: { text: string; isError: boolean } | null;
}

const defaultSettings: MirrorSettings = {
  preset_id: "gaming_usb",
  max_size: 1080,
  max_fps: 60,
  bitrate_mbps: 16,
  video_codec: "h264",
  render_driver: "direct3d",
  audio_mode: "output",
  audio_buffer_ms: 20,
  video_buffer_ms: 0,
  window_title: "TampilHP - Stream",
  borderless: false,
  always_on_top: false,
  turn_screen_off: false,
  stay_awake: true,
  no_control: false,
  record_path: "",
};

const state: AppState = {
  devices: [],
  activeSerials: [],
  presets: [],
  config: { settings: { ...defaultSettings }, saved_devices: [] },
  wirelessTab: "qr",
  qrSession: null,
  qrStatus: "",
  discovered: [],
  diagnostics: [],
  showDiagnostics: false,
  logs: [],
  busyAction: null,
  toast: null,
};

let toastTimer: number | null = null;
function renderToast() {
  const host = document.getElementById("toast-root");
  if (!host) return;
  host.innerHTML = state.toast
    ? `<div class="toast ${state.toast.isError ? "err" : ""}">${escapeHtml(state.toast.text)}</div>`
    : "";
}

function showToast(text: string, isError = false) {
  state.toast = { text, isError };
  renderToast();
  if (toastTimer) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    state.toast = null;
    renderToast();
  }, 4500);
}

function addLog(line: string) {
  state.logs.push(line);
  if (state.logs.length > 60) state.logs.shift();
  const logBox = document.getElementById("log-box");
  if (logBox) {
    logBox.innerHTML = state.logs.map(escapeHtml).join("<br>");
    logBox.scrollTop = logBox.scrollHeight;
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function refreshDevices() {
  try {
    const [devs, active] = await Promise.all([
      api.listDevices(),
      api.activeMirrors(),
    ]);
    state.devices = devs;
    state.activeSerials = active;
    renderDevicesSection();
    renderHeaderStatus();
  } catch (err) {
    addLog(`[ADB] ${String(err)}`);
  }
}

async function persistSettings() {
  try {
    await api.saveConfig(state.config);
  } catch (err) {
    showToast(`Gagal simpan setting: ${String(err)}`, true);
  }
}

async function handleMirrorToggle(serial: string) {
  const isRunning = state.activeSerials.includes(serial);
  if (isRunning) {
    await api.stopMirror(serial);
    showToast(`Mirror ${serial} dihentikan.`);
    await refreshDevices();
    return;
  }

  try {
    state.busyAction = `mirror:${serial}`;
    renderDevicesSection();
    const args = await api.startMirror(serial, state.config.settings);
    addLog(`[Launch] scrcpy ${args.join(" ")}`);
    showToast(`Mirroring dimulai (${serial})`);
    await refreshDevices();
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
    renderDevicesSection();
  }
}

async function handleEnableWirelessUsb(serial: string) {
  try {
    state.busyAction = `wifi:${serial}`;
    renderDevicesSection();
    const res = await api.enableWirelessUsb(serial);
    showToast(res.message);
    addLog(`[Wireless] ${serial} -> ${res.target_serial}`);
    await refreshDevices();
  } catch (err) {
    showToast(String(err), true);
    addLog(`[Wireless Error] ${String(err)}`);
  } finally {
    state.busyAction = null;
    renderDevicesSection();
  }
}

async function handleStartQr() {
  try {
    state.busyAction = "qr";
    renderWirelessSection();
    const session = await api.startQrPairing();
    state.qrSession = session;
    state.qrStatus =
      "Scan QR di HP: Developer Options -> Wireless Debugging -> Pair device with QR code.";
    renderWirelessSection();
    await drawQrCanvas(session.qr_payload);
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
  }
}

async function drawQrCanvas(payload: string) {
  const canvas = document.getElementById("qr-canvas") as HTMLCanvasElement | null;
  if (!canvas) return;
  await QRCode.toCanvas(canvas, payload, {
    width: 118,
    margin: 1,
    color: { dark: "#0b0f14", light: "#ffffff" },
  });
}

async function handlePairCode(e: Event) {
  e.preventDefault();
  const ip = (document.getElementById("pair-ip") as HTMLInputElement).value.trim();
  const pPort = Number((document.getElementById("pair-port") as HTMLInputElement).value);
  const code = (document.getElementById("pair-code") as HTMLInputElement).value.trim();
  const cPortRaw = (document.getElementById("connect-port") as HTMLInputElement).value.trim();
  const cPort = cPortRaw ? Number(cPortRaw) : null;

  if (!ip || !pPort || !code) {
    showToast("Isi IP, Pairing Port, dan Kode 6-digit.", true);
    return;
  }

  try {
    state.busyAction = "pair_code";
    renderWirelessSection();
    const target = await api.pairAndConnect(ip, pPort, code, cPort);
    showToast(`Terhubung wireless ke ${target}`);
    await refreshDevices();
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
    renderWirelessSection();
  }
}

async function handleDiscoverLan() {
  try {
    state.busyAction = "discover";
    renderWirelessSection();
    state.discovered = await api.discoverWireless();
    if (state.discovered.length === 0) {
      showToast(
        "Tidak ada perangkat Wireless Debugging terdeteksi di Wi-Fi. Pastikan Wireless Debugging di HP aktif."
      );
    }
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
    renderWirelessSection();
  }
}

async function handleManualConnect(e: Event) {
  e.preventDefault();
  const addr = (document.getElementById("manual-addr") as HTMLInputElement).value.trim();
  if (!addr) return;
  try {
    state.busyAction = "manual_connect";
    renderWirelessSection();
    const msg = await api.connectWireless(addr);
    showToast(msg);
    await refreshDevices();
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
    renderWirelessSection();
  }
}

async function handleRunDiagnostics(testIp: string | null = null) {
  state.showDiagnostics = true;
  state.busyAction = "diag";
  renderDiagnosticsModal();
  try {
    state.diagnostics = await api.runDiagnostics(testIp);
  } catch (err) {
    showToast(String(err), true);
  } finally {
    state.busyAction = null;
    renderDiagnosticsModal();
  }
}

function applyPreset(presetId: string) {
  const found = state.presets.find((p) => p.id === presetId);
  if (!found) return;
  state.config.settings = { ...found.settings };
  void persistSettings();
  renderPresetsSection();
  showToast(`Preset "${found.name}" dipilih.`);
}

function updateSetting<K extends keyof MirrorSettings>(key: K, value: MirrorSettings[K]) {
  state.config.settings[key] = value;
  void persistSettings();
}

function renderHeaderStatus() {
  const el = document.getElementById("header-status");
  if (!el) return;
  const readyCount = state.devices.filter((d) => d.state === "device").length;
  const mirrorCount = state.activeSerials.length;
  el.innerHTML = `
    <span class="pill ${readyCount > 0 ? "ok" : ""}">${readyCount} HP Siap</span>
    <span class="pill ${mirrorCount > 0 ? "ok" : ""}">${mirrorCount} Mirror Aktif</span>
  `;
}

function renderDevicesSection() {
  const box = document.getElementById("devices-container");
  if (!box) return;

  if (state.devices.length === 0) {
    box.innerHTML = `<div class="empty-state">Belum ada HP terdeteksi. Colok USB (USB Debugging aktif) atau pairing di panel Wireless di bawah.</div>`;
    return;
  }

  box.innerHTML = state.devices
    .map((d) => {
      const running = state.activeSerials.includes(d.serial);
      const isReady = d.state === "device";
      const badgeClass = isReady ? "ok" : d.state === "unauthorized" ? "err" : "warn";

      return `
        <div class="device-card ${running ? "active" : ""}">
          <div class="device-top">
            <div>
              <div class="device-name">${escapeHtml(d.model)}</div>
              <div class="device-serial">${escapeHtml(d.serial)}</div>
            </div>
            <div class="row-inline">
              <span class="pill">${d.transport === "usb" ? "USB" : "Wi-Fi"}</span>
              <span class="pill ${badgeClass}">${escapeHtml(d.state)}</span>
            </div>
          </div>
          ${
            d.state === "unauthorized"
              ? `<div class="hint-box err">Cek layar HP: centang <b>Always allow</b> lalu tekan <b>Allow</b>.</div>`
              : ""
          }
          <div class="device-actions">
            ${
              isReady
                ? `<button class="btn ${running ? "btn-danger" : "btn-primary"}" data-action="mirror" data-serial="${escapeHtml(d.serial)}" ${state.busyAction === `mirror:${d.serial}` ? "disabled" : ""}>
                    ${running ? "Stop Mirror" : "Mulai Mirror"}
                  </button>`
                : ""
            }
            ${
              isReady && d.transport === "usb"
                ? `<button class="btn btn-blue" data-action="to-wifi" data-serial="${escapeHtml(d.serial)}" ${state.busyAction === `wifi:${d.serial}` ? "disabled" : ""}>
                    ${state.busyAction === `wifi:${d.serial}` ? "Menghubungkan..." : "1-Klik Wi-Fi"}
                  </button>`
                : ""
            }
            ${
              isReady
                ? `<button class="btn" data-action="screenshot" data-serial="${escapeHtml(d.serial)}">Screenshot</button>`
                : ""
            }
            ${
              d.transport === "wifi"
                ? `<button class="btn btn-sm" data-action="disconnect" data-serial="${escapeHtml(d.serial)}">Putus</button>`
                : ""
            }
          </div>
        </div>
      `;
    })
    .join("");
}

function renderWirelessSection() {
  const box = document.getElementById("wireless-container");
  if (!box) return;

  const tabsHtml = `
    <div class="tabs">
      <button class="tab ${state.wirelessTab === "qr" ? "active" : ""}" data-wtab="qr">QR (Android 11+)</button>
      <button class="tab ${state.wirelessTab === "code" ? "active" : ""}" data-wtab="code">Kode Pair</button>
      <button class="tab ${state.wirelessTab === "manual" ? "active" : ""}" data-wtab="manual">Scan / IP</button>
    </div>
  `;

  let bodyHtml = "";
  if (state.wirelessTab === "qr") {
    bodyHtml = `
      <div style="display:flex;flex-direction:column;gap:8px;">
        <button class="btn btn-primary" id="btn-gen-qr" ${state.busyAction === "qr" ? "disabled" : ""}>
          ${state.qrSession ? "Buat QR Baru" : "Tampilkan QR Pairing"}
        </button>
        ${
          state.qrSession
            ? `
          <div class="qr-wrap">
            <div class="qr-box"><canvas id="qr-canvas"></canvas></div>
            <div style="font-size:11.5px;color:var(--muted);">
              <div style="color:var(--text);font-weight:600;margin-bottom:4px;">Langkah di HP:</div>
              1. Opsi Pengembang → Wireless Debugging<br/>
              2. Pilih <b>Pair device with QR code</b><br/>
              3. Arahkan kamera ke QR ini
            </div>
          </div>
          <div class="hint-box">${escapeHtml(state.qrStatus)}</div>
        `
            : `<div style="font-size:11.5px;color:var(--muted);">Pairing tanpa kabel sama sekali untuk Android 11+. Pastikan HP & PC di Wi-Fi yang sama.</div>`
        }
      </div>
    `;
  } else if (state.wirelessTab === "code") {
    bodyHtml = `
      <form id="form-pair-code" class="form-grid">
        <div class="field">
          <label>IP HP</label>
          <input class="input" id="pair-ip" placeholder="192.168.1.15" required />
        </div>
        <div class="field">
          <label>Pairing Port</label>
          <input class="input" id="pair-port" type="number" placeholder="37123" required />
        </div>
        <div class="field">
          <label>Kode 6-Digit</label>
          <input class="input" id="pair-code" placeholder="123456" required />
        </div>
        <div class="field">
          <label>Connect Port (opsional)</label>
          <input class="input" id="connect-port" type="number" placeholder="Auto via mDNS" />
        </div>
        <div class="field full">
          <button class="btn btn-primary" type="submit" ${state.busyAction === "pair_code" ? "disabled" : ""}>
            ${state.busyAction === "pair_code" ? "Pairing..." : "Pair & Hubungkan"}
          </button>
        </div>
      </form>
    `;
  } else {
    bodyHtml = `
      <div style="display:flex;flex-direction:column;gap:8px;">
        <form id="form-manual-connect" class="row-inline">
          <input class="input" id="manual-addr" placeholder="192.168.1.15:5555" />
          <button class="btn btn-primary" type="submit" ${state.busyAction === "manual_connect" ? "disabled" : ""}>Connect</button>
        </form>
        <button class="btn" id="btn-discover-lan" ${state.busyAction === "discover" ? "disabled" : ""}>
          ${state.busyAction === "discover" ? "Memindai Wi-Fi..." : "Scan HP di Jaringan (mDNS)"}
        </button>
        ${
          state.discovered.length > 0
            ? `<div style="display:flex;flex-direction:column;gap:5px;">
                ${state.discovered
                  .map(
                    (svc) => `
                  <div class="diag-item" style="align-items:center;">
                    <div>
                      <div style="font-weight:600;">${escapeHtml(svc.ip)}:${svc.port}</div>
                      <div style="font-size:11px;color:var(--muted);">${escapeHtml(svc.instance_name)}</div>
                    </div>
                    <button class="btn btn-sm btn-primary" data-action="connect-addr" data-addr="${escapeHtml(svc.ip)}:${svc.port}">Connect</button>
                  </div>
                `
                  )
                  .join("")}
              </div>`
            : ""
        }
      </div>
    `;
  }

  box.innerHTML = tabsHtml + bodyHtml;

  if (state.wirelessTab === "qr" && state.qrSession) {
    void drawQrCanvas(state.qrSession.qr_payload);
  }
}

function renderPresetsSection() {
  const box = document.getElementById("presets-container");
  if (!box) return;
  const s = state.config.settings;

  const presetsHtml = `
    <div class="preset-grid">
      ${state.presets
        .map(
          (p) => `
        <button class="preset-card ${s.preset_id === p.id ? "selected" : ""}" data-preset="${escapeHtml(p.id)}">
          <div class="preset-head">
            <span class="preset-name">${escapeHtml(p.name)}</span>
            <span class="pill ${s.preset_id === p.id ? "ok" : ""}">${escapeHtml(p.badge)}</span>
          </div>
          <div class="preset-desc">${escapeHtml(p.description)}</div>
        </button>
      `
        )
        .join("")}
    </div>
  `;

  const controlsHtml = `
    <div class="settings-grid" style="margin-top:12px;">
      <div class="field">
        <label>Resolusi Maks</label>
        <select class="select" id="set-max-size">
          <option value="0" ${s.max_size === 0 ? "selected" : ""}>Native (Asli)</option>
          <option value="1920" ${s.max_size === 1920 ? "selected" : ""}>1920p</option>
          <option value="1280" ${s.max_size === 1280 ? "selected" : ""}>1280p</option>
          <option value="1080" ${s.max_size === 1080 ? "selected" : ""}>1080p</option>
          <option value="720" ${s.max_size === 720 ? "selected" : ""}>720p (Ringan)</option>
        </select>
      </div>
      <div class="field">
        <label>Max FPS</label>
        <select class="select" id="set-max-fps">
          <option value="0" ${s.max_fps === 0 ? "selected" : ""}>Unlimited</option>
          <option value="120" ${s.max_fps === 120 ? "selected" : ""}>120 FPS</option>
          <option value="60" ${s.max_fps === 60 ? "selected" : ""}>60 FPS</option>
          <option value="30" ${s.max_fps === 30 ? "selected" : ""}>30 FPS</option>
        </select>
      </div>
      <div class="field">
        <label>Bitrate Video</label>
        <select class="select" id="set-bitrate">
          <option value="4" ${s.bitrate_mbps === 4 ? "selected" : ""}>4 Mbps (Hemat Wi-Fi)</option>
          <option value="8" ${s.bitrate_mbps === 8 ? "selected" : ""}>8 Mbps (Seimbang)</option>
          <option value="12" ${s.bitrate_mbps === 12 ? "selected" : ""}>12 Mbps (Stream)</option>
          <option value="16" ${s.bitrate_mbps === 16 ? "selected" : ""}>16 Mbps (USB Tajam)</option>
          <option value="24" ${s.bitrate_mbps === 24 ? "selected" : ""}>24 Mbps (Ultra)</option>
        </select>
      </div>
      <div class="field">
        <label>Video Codec</label>
        <select class="select" id="set-codec">
          <option value="h264" ${s.video_codec === "h264" ? "selected" : ""}>H.264 (Low Latency)</option>
          <option value="h265" ${s.video_codec === "h265" ? "selected" : ""}>H.265 (Hemat Bandwidth)</option>
          <option value="av1" ${s.video_codec === "av1" ? "selected" : ""}>AV1</option>
        </select>
      </div>
      <div class="field">
        <label>Render Driver (OBS/TikTok)</label>
        <select class="select" id="set-renderer">
          <option value="direct3d" ${s.render_driver === "direct3d" ? "selected" : ""}>Direct3D (OBS)</option>
          <option value="opengl" ${s.render_driver === "opengl" ? "selected" : ""}>OpenGL (TikTok Studio)</option>
          <option value="software" ${s.render_driver === "software" ? "selected" : ""}>Software (Anti Black Screen)</option>
          <option value="auto" ${s.render_driver === "auto" ? "selected" : ""}>Auto</option>
        </select>
      </div>
      <div class="field">
        <label>Audio Mode</label>
        <select class="select" id="set-audio">
          <option value="output" ${s.audio_mode === "output" ? "selected" : ""}>Forward ke PC</option>
          <option value="playback" ${s.audio_mode === "playback" ? "selected" : ""}>Playback + HP</option>
          <option value="none" ${s.audio_mode === "none" ? "selected" : ""}>Tanpa Audio</option>
        </select>
      </div>
      <div class="field">
        <label>Audio Buffer (ms)</label>
        <select class="select" id="set-audio-buf">
          <option value="15" ${s.audio_buffer_ms === 15 ? "selected" : ""}>15 ms (Ultra Low)</option>
          <option value="20" ${s.audio_buffer_ms === 20 ? "selected" : ""}>20 ms</option>
          <option value="35" ${s.audio_buffer_ms === 35 ? "selected" : ""}>35 ms (Stabil)</option>
          <option value="60" ${s.audio_buffer_ms === 60 ? "selected" : ""}>60 ms</option>
        </select>
      </div>
      <div class="field" style="grid-column: span 2;">
        <label>Judul Window (Agar Scene OBS / TikTok Studio Terkunci)</label>
        <input class="input" id="set-title" value="${escapeHtml(s.window_title)}" placeholder="TampilHP - Stream" />
      </div>
    </div>

    <div class="toggle-grid">
      <label class="toggle-item"><input type="checkbox" id="chk-screen-off" ${s.turn_screen_off ? "checked" : ""}/> Matikan Layar HP (Hemat Baterai)</label>
      <label class="toggle-item"><input type="checkbox" id="chk-stay-awake" ${s.stay_awake ? "checked" : ""}/> HP Tetap Nyala (Stay Awake)</label>
      <label class="toggle-item"><input type="checkbox" id="chk-borderless" ${s.borderless ? "checked" : ""}/> Window Borderless</label>
      <label class="toggle-item"><input type="checkbox" id="chk-ontop" ${s.always_on_top ? "checked" : ""}/> Always on Top</label>
      <label class="toggle-item"><input type="checkbox" id="chk-nocontrol" ${s.no_control ? "checked" : ""}/> View Only (Tanpa Klik/Keyboard)</label>
    </div>

    <div class="hint-box" style="margin-top:10px;">
      <b>Tips OBS / Streamlabs / TikTok Studio:</b> Gunakan <b>Window Capture</b> pada window <code>${escapeHtml(s.window_title || "TampilHP - Stream")}</code>. Jika layar capture hitam, ganti <b>Render Driver</b> ke <b>OpenGL</b> atau <b>Software</b> dan jangan minimize window mirror. Untuk audio terpisah, pakai <b>Application Audio Capture</b> ke <code>scrcpy.exe</code>.
    </div>
  `;

  box.innerHTML = presetsHtml + controlsHtml;
}

function renderDiagnosticsModal() {
  const box = document.getElementById("diag-container");
  if (!box) return;
  if (!state.showDiagnostics) {
    box.style.display = "none";
    return;
  }
  box.style.display = "block";
  box.innerHTML = `
    <div class="card-header">
      <span class="card-title">Diagnostik Koneksi & Wireless</span>
      <div class="row-inline">
        <input class="input" id="diag-ip-input" placeholder="Tes IP HP (opsional)" style="width:150px;" />
        <button class="btn btn-sm btn-primary" id="btn-recheck-diag">Tes Ulang</button>
        <button class="btn btn-sm" id="btn-close-diag">Tutup</button>
      </div>
    </div>
    <div class="diag-list">
      ${
        state.busyAction === "diag"
          ? `<div class="empty-state">Mengecek ADB, status device, VPN, dan koneksi...</div>`
          : state.diagnostics
              .map(
                (item) => `
            <div class="diag-item">
              <div>
                <div style="display:flex;align-items:center;gap:6px;margin-bottom:2px;">
                  <span class="pill ${item.status === "ok" ? "ok" : item.status === "warn" ? "warn" : "err"}">${item.status.toUpperCase()}</span>
                  <b>${escapeHtml(item.title)}</b>
                </div>
                <div style="font-size:11.5px;color:var(--muted);">${escapeHtml(item.detail)}</div>
              </div>
              ${
                item.fix_action === "clean_offline"
                  ? `<button class="btn btn-sm btn-blue" id="btn-fix-offline">Bersihkan</button>`
                  : item.fix_action === "restart_adb"
                  ? `<button class="btn btn-sm btn-danger" id="btn-fix-adb">Restart ADB</button>`
                  : ""
              }
            </div>
          `
              )
              .join("")
      }
    </div>
  `;
}

function render() {
  const root = document.getElementById("app");
  if (!root) return;

  root.innerHTML = `
    <div class="shell">
      <header class="topbar">
        <div class="brand">
          <div class="brand-mark">T</div>
          <div>
            <h1>TampilHP</h1>
          </div>
          <span class="pill">scrcpy 5.0.1</span>
        </div>
        <div class="topbar-actions">
          <div id="header-status" class="row-inline"></div>
          <button class="btn btn-sm" id="btn-clean-offline">Bersihkan Offline</button>
          <button class="btn btn-sm" id="btn-restart-adb">Restart ADB</button>
          <button class="btn btn-sm btn-blue" id="btn-open-diag">Diagnostik</button>
        </div>
      </header>

      <main class="main-grid">
        <div class="col">
          <section class="card">
            <div class="card-header">
              <span class="card-title">Perangkat Android</span>
              <button class="btn btn-sm" id="btn-refresh-dev">Refresh</button>
            </div>
            <div id="devices-container" class="device-list"></div>
          </section>

          <section class="card">
            <div class="card-header">
              <span class="card-title">Koneksi Wireless Tanpa Kabel</span>
            </div>
            <div id="wireless-container"></div>
          </section>
        </div>

        <div class="col">
          <section class="card">
            <div class="card-header">
              <span class="card-title">Preset & Pengaturan Low-Latency / Livestream</span>
            </div>
            <div id="presets-container"></div>
          </section>

          <section class="card" id="diag-container" style="display:none;"></section>

          <section class="card">
            <div class="card-header">
              <span class="card-title">Log Engine</span>
              <button class="btn btn-sm" id="btn-clear-log">Bersihkan</button>
            </div>
            <div class="log-box" id="log-box">${state.logs.map(escapeHtml).join("<br>")}</div>
          </section>
        </div>
      </main>
    </div>
    <div id="toast-root"></div>
  `;

  renderHeaderStatus();
  renderDevicesSection();
  renderWirelessSection();
  renderPresetsSection();
  renderDiagnosticsModal();
  renderToast();
}

// Event delegation on root
document.addEventListener("click", async (ev) => {
  const target = ev.target as HTMLElement | null;
  if (!target) return;

  const btn = target.closest("button") as HTMLButtonElement | null;
  if (!btn) return;

  if (btn.id === "btn-refresh-dev") {
    await refreshDevices();
    return;
  }
  if (btn.id === "btn-clean-offline") {
    const count = await api.cleanOffline();
    showToast(`Membersihkan ${count} koneksi offline.`);
    await refreshDevices();
    return;
  }
  if (btn.id === "btn-restart-adb") {
    const msg = await api.restartAdb();
    showToast(msg);
    await refreshDevices();
    return;
  }
  if (btn.id === "btn-open-diag") {
    await handleRunDiagnostics(null);
    return;
  }
  if (btn.id === "btn-close-diag") {
    state.showDiagnostics = false;
    renderDiagnosticsModal();
    return;
  }
  if (btn.id === "btn-recheck-diag") {
    const ipVal = (document.getElementById("diag-ip-input") as HTMLInputElement | null)?.value ?? null;
    await handleRunDiagnostics(ipVal);
    return;
  }
  if (btn.id === "btn-fix-offline") {
    await api.cleanOffline();
    await handleRunDiagnostics(null);
    await refreshDevices();
    return;
  }
  if (btn.id === "btn-fix-adb") {
    await api.restartAdb();
    await handleRunDiagnostics(null);
    await refreshDevices();
    return;
  }
  if (btn.id === "btn-clear-log") {
    state.logs = [];
    const logBox = document.getElementById("log-box");
    if (logBox) logBox.innerHTML = "";
    return;
  }
  if (btn.id === "btn-gen-qr") {
    await handleStartQr();
    return;
  }
  if (btn.id === "btn-discover-lan") {
    await handleDiscoverLan();
    return;
  }

  const wtab = btn.dataset.wtab as "qr" | "code" | "manual" | undefined;
  if (wtab) {
    state.wirelessTab = wtab;
    renderWirelessSection();
    return;
  }

  const presetId = btn.dataset.preset;
  if (presetId) {
    applyPreset(presetId);
    return;
  }

  const action = btn.dataset.action;
  const serial = btn.dataset.serial;
  if (action === "mirror" && serial) {
    await handleMirrorToggle(serial);
  } else if (action === "to-wifi" && serial) {
    await handleEnableWirelessUsb(serial);
  } else if (action === "screenshot" && serial) {
    try {
      const path = await api.takeScreenshot(serial);
      showToast(`Screenshot tersimpan: ${path}`);
    } catch (err) {
      showToast(String(err), true);
    }
  } else if (action === "disconnect" && serial) {
    await api.disconnectDevice(serial);
    showToast(`Koneksi ${serial} diputus.`);
    await refreshDevices();
  } else if (action === "connect-addr" && btn.dataset.addr) {
    try {
      const msg = await api.connectWireless(btn.dataset.addr);
      showToast(msg);
      await refreshDevices();
    } catch (err) {
      showToast(String(err), true);
    }
  }
});

document.addEventListener("submit", async (ev) => {
  const form = ev.target as HTMLFormElement;
  if (form.id === "form-pair-code") {
    await handlePairCode(ev);
  } else if (form.id === "form-manual-connect") {
    await handleManualConnect(ev);
  }
});

document.addEventListener("change", (ev) => {
  const el = ev.target as HTMLInputElement | HTMLSelectElement;
  switch (el.id) {
    case "set-max-size":
      updateSetting("max_size", Number(el.value));
      break;
    case "set-max-fps":
      updateSetting("max_fps", Number(el.value));
      break;
    case "set-bitrate":
      updateSetting("bitrate_mbps", Number(el.value));
      break;
    case "set-codec":
      updateSetting("video_codec", el.value);
      break;
    case "set-renderer":
      updateSetting("render_driver", el.value);
      break;
    case "set-audio":
      updateSetting("audio_mode", el.value);
      break;
    case "set-audio-buf":
      updateSetting("audio_buffer_ms", Number(el.value));
      break;
    case "set-title":
      updateSetting("window_title", el.value);
      break;
    case "chk-screen-off":
      updateSetting("turn_screen_off", (el as HTMLInputElement).checked);
      break;
    case "chk-stay-awake":
      updateSetting("stay_awake", (el as HTMLInputElement).checked);
      break;
    case "chk-borderless":
      updateSetting("borderless", (el as HTMLInputElement).checked);
      break;
    case "chk-ontop":
      updateSetting("always_on_top", (el as HTMLInputElement).checked);
      break;
    case "chk-nocontrol":
      updateSetting("no_control", (el as HTMLInputElement).checked);
      break;
  }
});

async function bootstrap() {
  try {
    const [presets, cfg] = await Promise.all([
      api.getPresets(),
      api.loadConfig(),
    ]);
    state.presets = presets;
    state.config = cfg;
  } catch (err) {
    addLog(`[Init] ${String(err)}`);
  }

  render();
  await refreshDevices();

  await listen<ScrcpyLogEvent>("scrcpy-log", (ev) => {
    addLog(`[${ev.payload.serial}] ${ev.payload.line}`);
  });

  await listen<ScrcpyExitEvent>("scrcpy-exit", (ev) => {
    addLog(`[Exit ${ev.payload.serial}] code=${ev.payload.code ?? "?"}`);
    if (ev.payload.hint) {
      showToast(ev.payload.hint, true);
    }
    void refreshDevices();
  });

  await listen<QrPairEvent>("qr-pair-status", (ev) => {
    state.qrStatus = ev.payload.message;
    addLog(`[QR Pair] ${ev.payload.message}`);
    if (ev.payload.status === "connected") {
      showToast(ev.payload.message);
      void refreshDevices();
    } else if (ev.payload.status === "error") {
      showToast(ev.payload.message, true);
    }
    renderWirelessSection();
  });

  window.setInterval(() => {
    void refreshDevices();
  }, 2500);
}

void bootstrap();
