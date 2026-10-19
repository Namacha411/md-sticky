import { getCurrentWindow } from "@tauri-apps/api/window";
import * as api from "./api";
import type { NoteMeta } from "./api";

const COLORS = ["#fff7a8", "#ffd6e7", "#c9f2d0", "#cfe5ff", "#e6d9ff", "#2b2b2b"];
const DELETE_CONFIRM_MS = 3000;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function applyStyle(meta: NoteMeta) {
  const root = document.documentElement.style;
  root.setProperty("--note-bg", meta.color);
  root.setProperty("--note-alpha", String(meta.alpha));
  // 暗い背景色のときは文字色を明るくする
  document.documentElement.classList.toggle("dark", isDark(meta.color));
}

function isDark(hex: string): boolean {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
  return 0.299 * r + 0.587 * g + 0.114 * b < 128;
}

export function initToolbar(meta: NoteMeta) {
  const win = getCurrentWindow();
  const update = (patch: api.MetaPatch) =>
    api.updateMeta(meta.id, patch).catch((e) => console.error("failed to update meta", e));
  applyStyle(meta);

  $("btn-new").addEventListener("click", () => api.createNote(meta.id));

  const palette = $("palette");
  for (const color of COLORS) {
    const swatch = document.createElement("button");
    swatch.className = "swatch";
    swatch.style.background = color;
    swatch.title = color;
    swatch.addEventListener("click", () => {
      meta.color = color;
      applyStyle(meta);
      update({ color });
      palette.hidden = true;
    });
    palette.append(swatch);
  }
  $("btn-color").addEventListener("click", () => (palette.hidden = !palette.hidden));

  const alpha = $<HTMLInputElement>("alpha");
  alpha.value = String(meta.alpha);
  alpha.addEventListener("input", () => {
    meta.alpha = Number(alpha.value);
    applyStyle(meta);
  });
  alpha.addEventListener("change", () => update({ alpha: meta.alpha }));

  const pin = $("btn-pin");
  pin.classList.toggle("active", meta.pinned);
  pin.addEventListener("click", async () => {
    meta.pinned = !meta.pinned;
    pin.classList.toggle("active", meta.pinned);
    await win.setAlwaysOnTop(meta.pinned);
    update({ pinned: meta.pinned });
  });

  // ダイアログを使わず、2 回押しで削除を確定する
  const del = $("btn-delete");
  let confirmTimer: number | undefined;
  del.addEventListener("click", () => {
    if (del.classList.contains("confirm")) {
      api.deleteNote(meta.id);
      return;
    }
    del.classList.add("confirm");
    del.title = "もう一度押すと削除";
    confirmTimer = window.setTimeout(() => {
      del.classList.remove("confirm");
      del.title = "削除";
    }, DELETE_CONFIRM_MS);
  });
  del.addEventListener("mouseleave", () => {
    clearTimeout(confirmTimer);
    del.classList.remove("confirm");
    del.title = "削除";
  });

  $("grip").addEventListener("mousedown", (e) => {
    e.preventDefault();
    win.startResizeDragging("SouthEast");
  });
}
