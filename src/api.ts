import { invoke } from "@tauri-apps/api/core";
import type {
  AdbDevice,
  AppConfig,
  DiagnosticItem,
  DiscoveredAdbService,
  MirrorSettings,
  PresetDefinition,
  QrSessionInfo,
  WirelessSwitchResult,
} from "./types";

export const api = {
  listDevices: () => invoke<AdbDevice[]>("cmd_list_devices"),
  enableWirelessUsb: (serial: string) =>
    invoke<WirelessSwitchResult>("cmd_enable_wireless_usb", { serial }),
  connectWireless: (address: string) =>
    invoke<string>("cmd_connect_wireless", { address }),
  pairAndConnect: (
    ip: string,
    pairPort: number,
    code: string,
    connectPort: number | null
  ) =>
    invoke<string>("cmd_pair_and_connect", {
      ip,
      pairPort,
      code,
      connectPort,
    }),
  startQrPairing: () => invoke<QrSessionInfo>("cmd_start_qr_pairing"),
  discoverWireless: () =>
    invoke<DiscoveredAdbService[]>("cmd_discover_wireless"),
  disconnectDevice: (serial: string) =>
    invoke<string>("cmd_disconnect_device", { serial }),
  cleanOffline: () => invoke<number>("cmd_clean_offline"),
  restartAdb: () => invoke<string>("cmd_restart_adb"),
  startMirror: (serial: string, settings: MirrorSettings) =>
    invoke<string[]>("cmd_start_mirror", { serial, settings }),
  stopMirror: (serial: string) => invoke<void>("cmd_stop_mirror", { serial }),
  activeMirrors: () => invoke<string[]>("cmd_active_mirrors"),
  getPresets: () => invoke<PresetDefinition[]>("cmd_get_presets"),
  loadConfig: () => invoke<AppConfig>("cmd_load_config"),
  saveConfig: (config: AppConfig) =>
    invoke<void>("cmd_save_config", { config }),
  runDiagnostics: (testIp: string | null) =>
    invoke<DiagnosticItem[]>("cmd_run_diagnostics", { testIp }),
  takeScreenshot: (serial: string) =>
    invoke<string>("cmd_take_screenshot", { serial }),
};
