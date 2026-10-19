import { listen } from "@tauri-apps/api/event";
import * as api from "./api";
import type { NoteSummary } from "./api";
import "./base.css";
import "./list.css";

const REFRESH_DELAY_MS = 150;
const DELETE_CONFIRM_MS = 3000;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const query = $<HTMLInputElement>("q");
const items = $("items");
const empty = $("empty");

let notes: NoteSummary[] = [];

function formatDate(ms: number): string {
  if (ms === 0) return "";
  const d = new Date(ms);
  const now = new Date();
  if (d.toDateString() === now.toDateString()) {
    return d.toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit" });
  }
  return d.toLocaleDateString("ja-JP", {
    year: d.getFullYear() === now.getFullYear() ? undefined : "numeric",
    month: "numeric",
    day: "numeric",
  });
}

/** text 中の needle (小文字化済み) に一致する部分を <mark> で囲んで el に追加する */
function appendHighlighted(el: HTMLElement, text: string, needle: string) {
  if (!needle) {
    el.textContent = text;
    return;
  }
  const lower = text.toLowerCase();
  let pos = 0;
  for (let hit = lower.indexOf(needle); hit !== -1; hit = lower.indexOf(needle, pos)) {
    el.append(text.slice(pos, hit));
    const mark = document.createElement("mark");
    mark.textContent = text.slice(hit, hit + needle.length);
    el.append(mark);
    pos = hit + needle.length;
  }
  el.append(text.slice(pos));
}

function renderItem(note: NoteSummary, needle: string): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "item";
  li.style.setProperty("--accent", note.color);
  li.tabIndex = 0;

  const meta = document.createElement("div");
  meta.className = "item-meta";
  const date = document.createElement("span");
  date.textContent = formatDate(note.modified);
  const del = document.createElement("button");
  del.className = "icon-btn item-delete";
  del.title = "削除";
  del.textContent = "";
  meta.append(date, del);

  const preview = document.createElement("p");
  preview.className = "item-preview";
  if (note.body.trim()) {
    appendHighlighted(preview, note.body.trim(), needle);
  } else {
    preview.textContent = "（空のメモ）";
    preview.classList.add("muted");
  }

  li.append(meta, preview);
  li.addEventListener("click", () => api.openNote(note.id));
  li.addEventListener("keydown", (e) => e.key === "Enter" && api.openNote(note.id));

  let confirmTimer: number | undefined;
  del.addEventListener("click", (e) => {
    e.stopPropagation();
    if (!del.classList.contains("confirm")) {
      del.classList.add("confirm");
      del.title = "もう一度押すと削除";
      confirmTimer = window.setTimeout(() => del.classList.remove("confirm"), DELETE_CONFIRM_MS);
      return;
    }
    clearTimeout(confirmTimer);
    li.remove();
    api.deleteNote(note.id);
  });
  return li;
}

function render() {
  const needle = query.value.trim().toLowerCase();
  const visible = needle ? notes.filter((n) => n.body.toLowerCase().includes(needle)) : notes;
  items.replaceChildren(...visible.map((n) => renderItem(n, needle)));
  empty.hidden = visible.length > 0;
  empty.textContent = needle ? "一致するメモはありません" : "メモはありません";
}

async function refresh() {
  notes = await api.listNotes();
  render();
}

let refreshTimer: number | undefined;
function scheduleRefresh() {
  clearTimeout(refreshTimer);
  refreshTimer = window.setTimeout(refresh, REFRESH_DELAY_MS);
}

query.addEventListener("input", render);
$("btn-new").addEventListener("click", () => api.createNote());
$("btn-settings").addEventListener("click", () => api.openSettings());
window.addEventListener("focus", scheduleRefresh);
listen(api.NOTES_CHANGED, scheduleRefresh);
refresh();
