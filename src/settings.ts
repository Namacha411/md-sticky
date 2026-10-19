import * as api from "./api";
import type { Settings } from "./api";
import { COLORS } from "./colors";
import { applyFontSettings } from "./fonts";
import "./base.css";
import "./settings.css";

/** フォント名の入力中に毎キー保存しないための待ち時間 */
const SAVE_DELAY_MS = 400;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const fontBody = $<HTMLInputElement>("font-body");
const fontCode = $<HTMLInputElement>("font-code");
const fontSize = $<HTMLInputElement>("font-size");
const fontSizeValue = $<HTMLOutputElement>("font-size-value");
const noteAlpha = $<HTMLInputElement>("note-alpha");
const noteAlphaValue = $<HTMLOutputElement>("note-alpha-value");
const swatches = $("swatches");
const autostart = $<HTMLInputElement>("autostart");
const preview = $("preview");
const error = $("error");

let settings: Settings;

function showError(e: unknown) {
  console.error(e);
  error.textContent = `保存できませんでした: ${e}`;
  error.hidden = false;
}

/** 入力欄・プレビューを settings に合わせる */
function render() {
  fontSizeValue.textContent = `${settings.fontSize}px`;
  noteAlphaValue.textContent = `${Math.round(settings.noteAlpha * 100)}%`;
  for (const swatch of swatches.children) {
    swatch.classList.toggle("selected", (swatch as HTMLElement).title === settings.noteColor);
  }
  preview.style.setProperty("--accent", settings.noteColor);
  applyFontSettings(settings, preview);
}

async function save() {
  saveTimer = undefined;
  try {
    settings = await api.updateSettings(settings);
    error.hidden = true;
  } catch (e) {
    showError(e);
  }
}

let saveTimer: number | undefined;
function saveLater() {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(save, SAVE_DELAY_MS);
}

/** 変更をすぐプレビューに出し、保存は少し待ってまとめて行う */
function edit(patch: Partial<Settings>, immediate = false) {
  settings = { ...settings, ...patch };
  render();
  if (immediate) {
    clearTimeout(saveTimer);
    save();
  } else {
    saveLater();
  }
}

async function main() {
  settings = await api.getSettings();
  fontBody.value = settings.fontBody;
  fontCode.value = settings.fontCode;
  fontSize.value = String(settings.fontSize);
  noteAlpha.value = String(settings.noteAlpha);

  for (const color of COLORS) {
    const swatch = document.createElement("button");
    swatch.className = "swatch";
    swatch.style.background = color;
    swatch.title = color;
    swatch.addEventListener("click", () => edit({ noteColor: color }, true));
    swatches.append(swatch);
  }
  render();

  fontBody.addEventListener("input", () => edit({ fontBody: fontBody.value.trim() }));
  fontCode.addEventListener("input", () => edit({ fontCode: fontCode.value.trim() }));
  fontSize.addEventListener("input", () => edit({ fontSize: Number(fontSize.value) }));
  noteAlpha.addEventListener("input", () => edit({ noteAlpha: Number(noteAlpha.value) }));

  autostart.addEventListener("change", async () => {
    try {
      await api.setAutostart(autostart.checked);
      error.hidden = true;
    } catch (e) {
      autostart.checked = !autostart.checked;
      showError(e);
    }
  });
  api.getAutostart().then(
    (enabled) => (autostart.checked = enabled),
    (e) => console.error("failed to read autostart", e),
  );

  // フォント一覧の取得は少し時間がかかるので、画面を出してから埋める
  api.listFonts().then(
    (fonts) =>
      $("fonts").replaceChildren(
        ...fonts.map((name) => Object.assign(document.createElement("option"), { value: name })),
      ),
    (e) => console.error("failed to list fonts", e),
  );
}

// 閉じる直前の入力も保存する
window.addEventListener("blur", () => {
  if (saveTimer !== undefined) {
    clearTimeout(saveTimer);
    save();
  }
});

main().catch((e) => {
  console.error(e);
  document.body.textContent = `md-sticky: ${e}`;
});
