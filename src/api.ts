import { invoke } from "@tauri-apps/api/core";

export interface NoteMeta {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** ヘッダーのアクセント色 */
  color: string;
  /** 本文背景の不透明度 */
  alpha: number;
}

export interface NoteSummary {
  id: string;
  color: string;
  body: string;
  /** 本文の最終更新時刻 (UNIX ミリ秒)。本文が無ければ 0 */
  modified: number;
}

/** アプリ全体の設定 */
export interface Settings {
  /** 本文のフォント名。空なら UI フォント */
  fontBody: string;
  /** コードのフォント名。空なら既定の等幅フォント */
  fontCode: string;
  /** 本文の文字サイズ (px) */
  fontSize: number;
  /** 新規付箋のアクセント色 */
  noteColor: string;
  /** 新規付箋の本文背景の不透明度 */
  noteAlpha: number;
}

export type MetaPatch = Partial<Pick<NoteMeta, "color" | "alpha">>;

/** メモの追加・更新・削除のたびに一覧ウィンドウへ送られるイベント */
export const NOTES_CHANGED = "notes-changed";

/** 設定が変わったときに全ウィンドウへ送られるイベント (payload は新しい Settings) */
export const SETTINGS_CHANGED = "settings-changed";

export const readNote = (id: string) => invoke<string>("read_note", { id });

export const saveNote = (id: string, content: string) =>
  invoke<void>("save_note", { id, content });

export const getMeta = (id: string) => invoke<NoteMeta>("get_meta", { id });

export const updateMeta = (id: string, patch: MetaPatch) =>
  invoke<void>("update_meta", { id, patch });

export const listNotes = () => invoke<NoteSummary[]>("list_notes");

export const createNote = (from?: string) => invoke<string>("create_note", { from });

export const openNote = (id: string) => invoke<void>("open_note", { id });

export const openList = () => invoke<void>("open_list");

export const openSettings = () => invoke<void>("open_settings");

export const deleteNote = (id: string) => invoke<void>("delete_note", { id });

export const getSettings = () => invoke<Settings>("get_settings");

/** 保存して、値を丸めた後の設定を返す */
export const updateSettings = (settings: Settings) =>
  invoke<Settings>("update_settings", { settings });

export const listFonts = () => invoke<string[]>("list_fonts");

export const getAutostart = () => invoke<boolean>("get_autostart");

export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });
