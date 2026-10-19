use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// ヘッダーのアクセント色。本文の背景は常に暗色
pub const DEFAULT_COLOR: &str = "#e6b905";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteMeta {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub color: String,
    pub alpha: f64,
}

impl NoteMeta {
    pub fn new(id: String, x: f64, y: f64) -> Self {
        Self {
            id,
            x,
            y,
            w: 280.0,
            h: 260.0,
            color: DEFAULT_COLOR.into(),
            alpha: 0.85,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct NotesFile {
    notes: Vec<NoteMeta>,
}

/// 付箋のメタ情報 (notes.json) と本文 (notes/{id}.md) を管理する。
pub struct Store {
    dir: PathBuf,
    pub notes: Vec<NoteMeta>,
    dirty: bool,
}

impl Store {
    pub fn load(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(dir.join("notes"))?;
        let notes = match fs::read_to_string(dir.join("notes.json")) {
            Ok(s) => serde_json::from_str::<NotesFile>(&s)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .notes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Self { dir, notes, dirty: false })
    }

    pub fn save_meta(&mut self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&NotesFile { notes: self.notes.clone() })
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_atomic(&self.dir.join("notes.json"), &json)?;
        self.dirty = false;
        Ok(())
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn get(&self, id: &str) -> Option<&NoteMeta> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut NoteMeta> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn add(&mut self, meta: NoteMeta) {
        self.notes.push(meta);
        self.dirty = true;
    }

    pub fn remove(&mut self, id: &str) -> io::Result<()> {
        self.notes.retain(|n| n.id != id);
        self.dirty = true;
        match fs::remove_file(self.body_path(id)?) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    pub fn read_body(&self, id: &str) -> io::Result<String> {
        match fs::read_to_string(self.body_path(id)?) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            r => r,
        }
    }

    /// 本文ファイルの最終更新時刻。まだ本文が無ければ None
    pub fn body_modified(&self, id: &str) -> Option<SystemTime> {
        fs::metadata(self.body_path(id).ok()?).ok()?.modified().ok()
    }

    pub fn write_body(&self, id: &str, content: &str) -> io::Result<()> {
        write_atomic(&self.body_path(id)?, content)
    }

    fn body_path(&self, id: &str) -> io::Result<PathBuf> {
        // id はファイル名に使うので、UUID の文字以外は拒否してパストラバーサルを防ぐ
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid note id"));
        }
        Ok(self.dir.join("notes").join(format!("{id}.md")))
    }
}

/// 一時ファイルに書いてから置き換え、書き込み途中で落ちてもファイルが壊れないようにする。
fn write_atomic(path: &Path, content: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("md-sticky-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn meta_and_body_round_trip() {
        let dir = temp_dir("roundtrip");
        let mut store = Store::load(dir.clone()).unwrap();
        assert!(store.notes.is_empty());

        let mut meta = NoteMeta::new("0a1b-2c3d".into(), 10.0, 20.0);
        meta.alpha = 0.5;
        store.add(meta.clone());
        store.write_body(&meta.id, "# 見出し\n\n- [ ] task").unwrap();
        store.save_meta().unwrap();
        assert!(!store.is_dirty());

        let reloaded = Store::load(dir.clone()).unwrap();
        assert_eq!(reloaded.notes, vec![meta.clone()]);
        assert_eq!(reloaded.read_body(&meta.id).unwrap(), "# 見出し\n\n- [ ] task");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn remove_deletes_body_and_missing_body_is_empty() {
        let dir = temp_dir("remove");
        let mut store = Store::load(dir.clone()).unwrap();
        store.add(NoteMeta::new("abc".into(), 0.0, 0.0));
        store.write_body("abc", "hello").unwrap();
        store.remove("abc").unwrap();
        assert!(store.get("abc").is_none());
        assert_eq!(store.read_body("abc").unwrap(), "");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_path_traversal_id() {
        let dir = temp_dir("traversal");
        let store = Store::load(dir.clone()).unwrap();
        assert!(store.read_body("../secret").is_err());
        assert!(store.write_body("..\\x", "x").is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
