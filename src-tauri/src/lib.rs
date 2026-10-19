mod store;
mod window;

use serde::Deserialize;
use std::sync::Mutex;
use std::time::Duration;
use store::{NoteMeta, Store};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager, RunEvent, State, WindowEvent};

type StoreState = Mutex<Store>;

fn to_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn read_note(store: State<'_, StoreState>, id: String) -> Result<String, String> {
    store.lock().unwrap().read_body(&id).map_err(to_err)
}

#[tauri::command]
fn save_note(store: State<'_, StoreState>, id: String, content: String) -> Result<(), String> {
    store.lock().unwrap().write_body(&id, &content).map_err(to_err)
}

#[tauri::command]
fn get_meta(store: State<'_, StoreState>, id: String) -> Result<NoteMeta, String> {
    store.lock().unwrap().get(&id).cloned().ok_or_else(|| format!("note not found: {id}"))
}

#[derive(Deserialize)]
struct MetaPatch {
    color: Option<String>,
    alpha: Option<f64>,
    pinned: Option<bool>,
}

#[tauri::command]
fn update_meta(store: State<'_, StoreState>, id: String, patch: MetaPatch) -> Result<(), String> {
    let mut store = store.lock().unwrap();
    let meta = store.get_mut(&id).ok_or_else(|| format!("note not found: {id}"))?;
    if let Some(color) = patch.color {
        meta.color = color;
    }
    if let Some(alpha) = patch.alpha {
        meta.alpha = alpha.clamp(0.0, 1.0);
    }
    if let Some(pinned) = patch.pinned {
        meta.pinned = pinned;
    }
    store.mark_dirty();
    Ok(())
}

// Windows ではウィンドウ生成を同期コマンド内で行うとデッドロックするため async にする
#[tauri::command]
async fn create_note(app: AppHandle, from: Option<String>) -> Result<String, String> {
    new_note(&app, from.as_deref()).map_err(to_err)
}

#[tauri::command]
async fn delete_note(app: AppHandle, id: String) -> Result<(), String> {
    {
        let state = app.state::<StoreState>();
        let mut store = state.lock().unwrap();
        store.remove(&id).map_err(to_err)?;
        store.save_meta().map_err(to_err)?;
    }
    if let Some(w) = app.get_webview_window(&window::label(&id)) {
        // close() だと閉じる前の保存処理が走って本文が復活しうるので destroy() を使う
        w.destroy().map_err(to_err)?;
    }
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
        let meta = NoteMeta::new(uuid::Uuid::new_v4().to_string(), x, y);
        store.add(meta.clone());
        store.save_meta()?;
        meta
    };
    // ストアのロックを外してから開く (ウィンドウイベントがロックを取りに来るため)
    window::open_note_window(app, &meta)?;
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

fn build_tray(app: &App) -> tauri::Result<()> {
    let new = MenuItem::with_id(app, "new", "新規付箋", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "すべて表示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&new, &show, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("md-sticky")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "new" => {
                if let Err(e) = new_note(app, None) {
                    eprintln!("failed to create note: {e}");
                }
            }
            "show" => show_all(app),
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
        .invoke_handler(tauri::generate_handler![
            read_note,
            save_note,
            get_meta,
            update_meta,
            create_note,
            delete_note
        ])
        .setup(|app| {
            let mut store = Store::load(app.path().app_data_dir()?)?;
            if store.notes.is_empty() {
                store.add(NoteMeta::new(uuid::Uuid::new_v4().to_string(), 100.0, 100.0));
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
