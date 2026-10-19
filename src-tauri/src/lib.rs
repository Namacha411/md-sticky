mod store;
mod window;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};
use store::{NoteMeta, Settings, Store};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;

type StoreState = Mutex<Store>;

/// メモの内容や構成が変わったことを一覧ウィンドウに知らせるイベント
const NOTES_CHANGED: &str = "notes-changed";
/// 設定が変わったことを全ウィンドウに知らせるイベント (payload は新しい Settings)
const SETTINGS_CHANGED: &str = "settings-changed";

fn to_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn notify_changed(app: &AppHandle) {
    if let Err(e) = app.emit_to(window::LIST_LABEL, NOTES_CHANGED, ()) {
        eprintln!("failed to emit {NOTES_CHANGED}: {e}");
    }
}

#[tauri::command]
fn read_note(store: State<'_, StoreState>, id: String) -> Result<String, String> {
    store.lock().unwrap().read_body(&id).map_err(to_err)
}

#[tauri::command]
fn save_note(
    app: AppHandle,
    store: State<'_, StoreState>,
    id: String,
    content: String,
) -> Result<(), String> {
    {
        let store = store.lock().unwrap();
        // 削除直後に遅れて届いた保存で本文が復活しないよう、存在しないメモへの保存は捨てる
        if store.get(&id).is_none() {
            return Ok(());
        }
        store.write_body(&id, &content).map_err(to_err)?;
    }
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
fn get_meta(store: State<'_, StoreState>, id: String) -> Result<NoteMeta, String> {
    store.lock().unwrap().get(&id).cloned().ok_or_else(|| format!("note not found: {id}"))
}

#[derive(Deserialize)]
struct MetaPatch {
    color: Option<String>,
    alpha: Option<f64>,
}

#[tauri::command]
fn update_meta(
    app: AppHandle,
    store: State<'_, StoreState>,
    id: String,
    patch: MetaPatch,
) -> Result<(), String> {
    {
        let mut store = store.lock().unwrap();
        let meta = store.get_mut(&id).ok_or_else(|| format!("note not found: {id}"))?;
        if let Some(color) = patch.color {
            meta.color = color;
        }
        if let Some(alpha) = patch.alpha {
            meta.alpha = alpha.clamp(0.0, 1.0);
        }
        store.mark_dirty();
    }
    notify_changed(&app);
    Ok(())
}

#[derive(Serialize)]
struct NoteSummary {
    id: String,
    color: String,
    body: String,
    /// 本文の最終更新時刻 (UNIX ミリ秒)。本文が無ければ 0
    modified: u64,
}

#[tauri::command]
fn list_notes(store: State<'_, StoreState>) -> Result<Vec<NoteSummary>, String> {
    let store = store.lock().unwrap();
    let mut notes = store
        .notes
        .iter()
        .map(|m| {
            Ok(NoteSummary {
                id: m.id.clone(),
                color: m.color.clone(),
                body: store.read_body(&m.id)?,
                modified: store
                    .body_modified(&m.id)
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_millis() as u64),
            })
        })
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(to_err)?;
    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(notes)
}

#[tauri::command]
fn get_settings(store: State<'_, StoreState>) -> Settings {
    store.lock().unwrap().settings.clone()
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    store: State<'_, StoreState>,
    settings: Settings,
) -> Result<Settings, String> {
    let saved = {
        let mut store = store.lock().unwrap();
        store.save_settings(settings).map_err(to_err)?;
        store.settings.clone()
    };
    if let Err(e) = app.emit(SETTINGS_CHANGED, &saved) {
        eprintln!("failed to emit {SETTINGS_CHANGED}: {e}");
    }
    Ok(saved)
}

/// インストール済みフォントのファミリー名 (重複なし・名前順)
#[tauri::command]
async fn list_fonts() -> Result<Vec<String>, String> {
    let mut families = font_kit::source::SystemSource::new().all_families().map_err(to_err)?;
    // 縦書き用 (@ 始まり) のファミリーは選択肢に出さない
    families.retain(|f| !f.starts_with('@'));
    families.sort_by_key(|f| f.to_lowercase());
    families.dedup();
    Ok(families)
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(to_err)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled { autolaunch.enable() } else { autolaunch.disable() }.map_err(to_err)
}

// Windows ではウィンドウ操作を同期コマンド内で行うとデッドロックしうるため、以下は async にする

#[tauri::command]
async fn create_note(app: AppHandle, from: Option<String>) -> Result<String, String> {
    new_note(&app, from.as_deref()).map_err(to_err)
}

#[tauri::command]
async fn open_note(app: AppHandle, id: String) -> Result<(), String> {
    let meta = app
        .state::<StoreState>()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("note not found: {id}"))?;
    window::open_note_window(&app, &meta).map_err(to_err)?;
    Ok(())
}

#[tauri::command]
async fn open_list(app: AppHandle) -> Result<(), String> {
    window::open_list_window(&app).map_err(to_err)?;
    Ok(())
}

#[tauri::command]
async fn open_settings(app: AppHandle) -> Result<(), String> {
    window::open_settings_window(&app).map_err(to_err)?;
    Ok(())
}

/// メモを削除する。付箋ウィンドウが開いていれば閉じるが、呼び出し元自身のウィンドウは
/// 応答を受け取れるよう残し、呼び出し元に破棄を任せる。
#[tauri::command]
async fn delete_note(app: AppHandle, caller: WebviewWindow, id: String) -> Result<(), String> {
    {
        let state = app.state::<StoreState>();
        let mut store = state.lock().unwrap();
        store.remove(&id).map_err(to_err)?;
        store.save_meta().map_err(to_err)?;
    }
    let label = window::label(&id);
    if caller.label() != label {
        if let Some(w) = app.get_webview_window(&label) {
            w.destroy().map_err(to_err)?;
        }
    }
    notify_changed(&app);
    Ok(())
}

/// 付箋を新規作成して開く。`from` があればその付箋から少しずらした位置に置く。
fn new_note(app: &AppHandle, from: Option<&str>) -> tauri::Result<String> {
    let meta = {
        let state = app.state::<StoreState>();
        let mut store = state.lock().unwrap();
        let (x, y) = from
            .and_then(|id| store.get(id))
            .map(|m| (m.x + 30.0, m.y + 30.0))
            .unwrap_or((100.0, 100.0));
        let meta = NoteMeta::new(uuid::Uuid::new_v4().to_string(), x, y, &store.settings);
        store.add(meta.clone());
        store.save_meta()?;
        meta
    };
    // ストアのロックを外してから開く (ウィンドウイベントがロックを取りに来るため)
    window::open_note_window(app, &meta)?;
    notify_changed(app);
    Ok(meta.id)
}

fn show_all(app: &AppHandle) {
    let metas = app.state::<StoreState>().lock().unwrap().notes.clone();
    for meta in &metas {
        if let Err(e) = window::open_note_window(app, meta) {
            eprintln!("failed to open note {}: {e}", meta.id);
        }
    }
}

fn open_list_or_log(app: &AppHandle) {
    if let Err(e) = window::open_list_window(app) {
        eprintln!("failed to open list: {e}");
    }
}

fn build_tray(app: &App) -> tauri::Result<()> {
    let new = MenuItem::with_id(app, "new", "新規付箋", true, None::<&str>)?;
    let list = MenuItem::with_id(app, "list", "メモの一覧", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "すべて表示", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "設定", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&new, &list, &show, &settings, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("md-sticky")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_list_or_log(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "new" => {
                if let Err(e) = new_note(app, None) {
                    eprintln!("failed to create note: {e}");
                }
            }
            "list" => open_list_or_log(app),
            "show" => show_all(app),
            "settings" => {
                if let Err(e) = window::open_settings_window(app) {
                    eprintln!("failed to open settings: {e}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// 移動・リサイズのたびに書き込まないよう、変更があったときだけ 1 秒おきに notes.json を保存する。
fn spawn_meta_saver(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let state = app.state::<StoreState>();
        let mut store = state.lock().unwrap();
        if store.is_dirty() {
            if let Err(e) = store.save_meta() {
                eprintln!("failed to save notes.json: {e}");
            }
        }
    });
}

fn track_geometry(window: &tauri::Window, event: &WindowEvent) {
    let Some(id) = window::id_from_label(window.label()) else { return };
    let Ok(scale) = window.scale_factor() else { return };
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    let state = window.state::<StoreState>();
    let mut store = state.lock().unwrap();
    let Some(meta) = store.get_mut(id) else { return };
    match event {
        WindowEvent::Moved(pos) => {
            let pos = pos.to_logical::<f64>(scale);
            meta.x = pos.x;
            meta.y = pos.y;
        }
        WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
            let size = size.to_logical::<f64>(scale);
            meta.w = size.width;
            meta.h = size.height;
        }
        _ => return,
    }
    store.mark_dirty();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_all(app)))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            read_note,
            save_note,
            get_meta,
            update_meta,
            list_notes,
            create_note,
            open_note,
            open_list,
            open_settings,
            delete_note,
            get_settings,
            update_settings,
            list_fonts,
            get_autostart,
            set_autostart
        ])
        .setup(|app| {
            let mut store = Store::load(app.path().app_data_dir()?)?;
            if store.notes.is_empty() {
                let meta = NoteMeta::new(uuid::Uuid::new_v4().to_string(), 100.0, 100.0, &store.settings);
                store.add(meta);
                store.save_meta()?;
            }
            let metas = store.notes.clone();
            app.manage::<StoreState>(Mutex::new(store));
            for meta in &metas {
                window::open_note_window(app.handle(), meta)?;
            }
            build_tray(app)?;
            spawn_meta_saver(app.handle().clone());
            Ok(())
        })
        .on_window_event(track_geometry)
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| match event {
        // 最後の付箋を閉じても常駐し続ける (トレイの「終了」は code 付きで来る)
        RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
        RunEvent::Exit => {
            if let Err(e) = app.state::<StoreState>().lock().unwrap().save_meta() {
                eprintln!("failed to save notes.json: {e}");
            }
        }
        _ => {}
    });
}
