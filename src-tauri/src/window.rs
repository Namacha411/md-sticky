use crate::store::NoteMeta;
use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const LABEL_PREFIX: &str = "note-";
pub const LIST_LABEL: &str = "list";

pub fn label(id: &str) -> String {
    format!("{LABEL_PREFIX}{id}")
}

pub fn id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(LABEL_PREFIX)
}

/// 既にあるウィンドウなら表示して前面に出す。
fn reveal(app: &AppHandle, label: &str) -> tauri::Result<Option<WebviewWindow>> {
    let Some(w) = app.get_webview_window(label) else { return Ok(None) };
    w.unminimize()?;
    w.show()?;
    w.set_focus()?;
    Ok(Some(w))
}

/// 付箋ウィンドウを開く。既に存在する場合は表示して前面に出す。
pub fn open_note_window(app: &AppHandle, meta: &NoteMeta) -> tauri::Result<WebviewWindow> {
    if let Some(w) = reveal(app, &label(&meta.id))? {
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
    .always_on_top(true)
    .skip_taskbar(true)
    .position(meta.x, meta.y)
    .inner_size(meta.w, meta.h)
    .min_inner_size(180.0, 120.0)
    .build()
}

/// メモの一覧ウィンドウを開く。
pub fn open_list_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = reveal(app, LIST_LABEL)? {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, LIST_LABEL, WebviewUrl::App("list.html".into()))
        .title("メモの一覧")
        .theme(Some(Theme::Dark))
        .inner_size(360.0, 560.0)
        .min_inner_size(260.0, 300.0)
        .build()
}
