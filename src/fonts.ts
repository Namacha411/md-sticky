import { listen } from "@tauri-apps/api/event";
import * as api from "./api";
import type { Settings } from "./api";

/** 設定のフォント・文字サイズを CSS 変数に反映する。フォント名が空なら既定のフォントに戻す */
export function applyFontSettings(settings: Settings, root: HTMLElement = document.documentElement) {
  const setFont = (name: string, font: string, fallback: string) =>
    font
      ? root.style.setProperty(name, `"${font}", ${fallback}`)
      : root.style.removeProperty(name);
  setFont("--note-font", settings.fontBody, "var(--font-ui)");
  setFont("--note-font-code", settings.fontCode, "var(--font-code)");
  root.style.setProperty("--note-font-size", `${settings.fontSize}px`);
}

/** 現在の設定を反映し、以後の変更にも追従する */
export async function followFontSettings() {
  await listen<Settings>(api.SETTINGS_CHANGED, (e) => applyFontSettings(e.payload));
  applyFontSettings(await api.getSettings());
}
