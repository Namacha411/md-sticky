# md-sticky

Markdown で書ける、背景が半透明のデスクトップ付箋アプリ（Tauri 2 + Milkdown）。

- 1付箋 = 1ウィンドウ。枠なし・背景だけ半透明・最前面固定の切り替え
- Milkdown Crepe による WYSIWYG 風の Markdown 編集
- 本文は自動保存され、位置・サイズ・色は次回起動時に復元される
- トレイに常駐（新規付箋 / すべて表示 / 終了）

## 操作

| 操作 | 方法 |
|---|---|
| 移動 | 上端のバーをドラッグ |
| リサイズ | 右下のグリップをドラッグ |
| 新規 | ＋ / `Ctrl+N` / トレイメニュー |
| 隠す | `Ctrl+W`（トレイの「すべて表示」で再表示） |
| 削除 | 🗑 を 2 回押す |
| 色・不透明度・最前面 | 🎨 / スライダー / 📌 |

## 保存先

`%APPDATA%\com.thdyk.md-sticky\`

- `notes/{id}.md` … 本文（素の Markdown）
- `notes.json` … 位置・サイズ・色・不透明度・最前面設定

## 開発

```sh
bun install
bun tauri dev      # 開発起動
bun tauri build    # インストーラー作成
cd src-tauri && cargo test
```
