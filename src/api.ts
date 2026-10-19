import { invoke } from "@tauri-apps/api/core";

export interface NoteMeta {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  alpha: number;
  pinned: boolean;
}

export type MetaPatch = Partial<Pick<NoteMeta, "color" | "alpha" | "pinned">>;

export const readNote = (id: string) => invoke<string>("read_note", { id });

export const saveNote = (id: string, content: string) =>
  invoke<void>("save_note", { id, content });

export const getMeta = (id: string) => invoke<NoteMeta>("get_meta", { id });

export const updateMeta = (id: string, patch: MetaPatch) =>
  invoke<void>("update_meta", { id, patch });

export const createNote = (from?: string) => invoke<string>("create_note", { from });

export const deleteNote = (id: string) => invoke<void>("delete_note", { id });
