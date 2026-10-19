import * as api from "./api";
import { createEditor } from "./editor";
import { followFontSettings } from "./fonts";
import { hideNote, initHeader } from "./header";
import "./base.css";
import "./style.css";

async function main() {
  const id = new URLSearchParams(location.search).get("id");
  if (!id) throw new Error("note id is missing in URL");

  const [meta, body] = await Promise.all([api.getMeta(id), api.readNote(id), followFontSettings()]);
  const editor = await createEditor(document.getElementById("editor")!, body, (md) =>
    api.saveNote(id, md),
  );
  initHeader(meta, editor);

  // トレイから終了したときなどに未保存分を失わないよう、フォーカスが外れたら即保存
  window.addEventListener("blur", editor.flush);

  window.addEventListener("keydown", (e) => {
    if (!e.ctrlKey) return;
    if (e.key === "n") {
      e.preventDefault();
      api.createNote(id);
    } else if (e.key === "w") {
      e.preventDefault();
      hideNote(editor);
    }
  });
}

main().catch((e) => {
  console.error(e);
  document.body.textContent = `md-sticky: ${e}`;
});
