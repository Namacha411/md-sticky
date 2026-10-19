use crate::store::NoteMeta;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const LABEL_PREFIX: &str = "note-";

pub fn label(id: &str) -> String {
    format!("{LABEL_PREFIX}{id}")
}

pub fn id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(LABEL_PREFIX)
}

/// 付箋ウィンドウを開く。既に存在する場合は表示して前面に出す。
pub fn open_note_window(app: &AppHandle, meta: &NoteMeta) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(&label(&meta.id)) {
        w.show()?;
        w.set_focus()?;
        return Ok(w);
    }
    WebviewWindowBuilder::new(
        app,
        label(&meta.id),
        WebviewUrl::App(format!("index.html?id={}", meta.id).into()),
    )
    .title("md-sticky")
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(meta.pinned)
    .skip_taskbar(true)
    .position(meta.x, meta.y)
    .inner_size(meta.w, meta.h)
    .min_inner_size(160.0, 120.0)
    .build()
}
