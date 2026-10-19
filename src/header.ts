import { getCurrentWindow } from "@tauri-apps/api/window";
import * as api from "./api";
import type { NoteMeta } from "./api";
import { COLORS, foregroundOn } from "./colors";
import type { NoteEditor } from "./editor";

const DELETE_CONFIRM_MS = 3000;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function applyStyle(meta: NoteMeta) {
  const root = document.documentElement.style;
  root.setProperty("--accent", meta.color);
  root.setProperty("--on-accent", foregroundOn(meta.color));
  root.setProperty("--note-alpha", String(meta.alpha));
}

/** 付箋を隠す。データは残り、一覧やトレイから再表示できる */
export function hideNote(editor: NoteEditor) {
  editor.flush();
  getCurrentWindow().hide();
}

export function initHeader(meta: NoteMeta, editor: NoteEditor) {
  const win = getCurrentWindow();
  const update = (patch: api.MetaPatch) =>
    api.updateMeta(meta.id, patch).catch((e) => console.error("failed to update meta", e));
  applyStyle(meta);

  $("btn-new").addEventListener("click", () => api.createNote(meta.id));
  $("btn-close").addEventListener("click", () => hideNote(editor));

  // ---- メニュー ----
  const menu = $("menu");
  const menuButton = $("btn-menu");
  const deleteButton = $("btn-delete");
  const deleteLabel = $("delete-label");
  let confirmTimer: number | undefined;

  const resetDelete = () => {
    clearTimeout(confirmTimer);
    deleteButton.classList.remove("confirm");
    deleteLabel.textContent = "メモの削除";
  };
  const closeMenu = () => {
    menu.hidden = true;
    resetDelete();
  };

  menuButton.addEventListener("click", () => (menu.hidden ? (menu.hidden = false) : closeMenu()));
  document.addEventListener("mousedown", (e) => {
    const target = e.target as Node;
    if (!menu.hidden && !menu.contains(target) && !menuButton.contains(target)) closeMenu();
  });
  document.addEventListener("keydown", (e) => e.key === "Escape" && closeMenu());
  window.addEventListener("blur", closeMenu);

  const swatches = $("swatches");
  for (const color of COLORS) {
    const swatch = document.createElement("button");
    swatch.className = "swatch";
    swatch.style.background = color;
    swatch.title = color;
    swatch.addEventListener("click", () => {
      meta.color = color;
      applyStyle(meta);
      update({ color });
    });
    swatches.append(swatch);
  }

  const alpha = $<HTMLInputElement>("alpha");
  alpha.value = String(meta.alpha);
  alpha.addEventListener("input", () => {
    meta.alpha = Number(alpha.value);
    applyStyle(meta);
  });
  alpha.addEventListener("change", () => update({ alpha: meta.alpha }));

  $("btn-list").addEventListener("click", () => {
    closeMenu();
    api.openList();
  });

  // ダイアログを使わず、2 回押しで削除を確定する
  deleteButton.addEventListener("click", async () => {
    if (!deleteButton.classList.contains("confirm")) {
      deleteButton.classList.add("confirm");
      deleteLabel.textContent = "もう一度押すと削除";
      confirmTimer = window.setTimeout(resetDelete, DELETE_CONFIRM_MS);
      return;
    }
    // 先に隠して即座に消えたように見せ、後片付けは裏で行う
    editor.discard();
    await win.hide();
    await api.deleteNote(meta.id);
    await win.destroy();
  });

  $("grip").addEventListener("mousedown", (e) => {
    e.preventDefault();
    win.startResizeDragging("SouthEast");
  });
}
