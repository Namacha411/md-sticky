import { Crepe } from "@milkdown/crepe";
import { editorViewCtx } from "@milkdown/kit/core";
import { lift } from "@milkdown/kit/prose/commands";
import { type EditorState, TextSelection } from "@milkdown/kit/prose/state";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame-dark.css";

const SAVE_DELAY_MS = 500;

export interface NoteEditor {
  /** 保存待ちの変更があれば即座に保存する */
  flush: () => void;
  /** 保存待ちの変更を捨てる (削除時に本文が復活しないようにする) */
  discard: () => void;
}

export async function createEditor(
  root: HTMLElement,
  initial: string,
  save: (markdown: string) => Promise<void>,
): Promise<NoteEditor> {
  const crepe = new Crepe({
    root,
    defaultValue: initial,
    features: {
      // 付箋の幅では左側のドラッグハンドルが邪魔なので切る。画像・数式も付箋には不要
      [Crepe.Feature.BlockEdit]: false,
      [Crepe.Feature.ImageBlock]: false,
      [Crepe.Feature.Latex]: false,
    },
    featureConfigs: {
      [Crepe.Feature.Placeholder]: { text: "メモを作成する...", mode: "doc" },
    },
  });

  let pending: string | null = null;
  let timer: number | undefined;

  const discard = () => {
    clearTimeout(timer);
    pending = null;
  };

  const flush = () => {
    if (pending === null) return;
    const markdown = pending;
    discard();
    save(markdown).catch((e) => console.error("failed to save note", e));
  };

  crepe.on((listener) => {
    listener.markdownUpdated((_ctx, markdown) => {
      pending = markdown;
      clearTimeout(timer);
      timer = window.setTimeout(flush, SAVE_DELAY_MS);
    });
  });

  await crepe.create();
  liftBlockquoteOnBackspace(root, crepe);
  return { flush, discard };
}

/** カーソルが引用ブロック先頭の段落の、さらに先頭にあるか */
function atStartOfBlockquote(state: EditorState): boolean {
  const sel = state.selection;
  if (!(sel instanceof TextSelection) || !sel.empty) return false;
  const $pos = sel.$from;
  if ($pos.parentOffset !== 0 || $pos.depth < 1) return false;
  return $pos.node($pos.depth - 1).type.name === "blockquote" && $pos.index($pos.depth - 1) === 0;
}

/**
 * Milkdown は Backspace を joinTextblockBackward (前のブロックとの結合のみ) に差し替えているため、
 * 文書先頭の引用ブロックでは何も起きない。引用の先頭で Backspace を押したら引用を解除する。
 */
function liftBlockquoteOnBackspace(root: HTMLElement, crepe: Crepe) {
  // ProseMirror より先に処理するため、祖先要素のキャプチャフェーズで捕まえる
  root.addEventListener(
    "keydown",
    (e) => {
      if (e.key !== "Backspace" || e.isComposing || e.ctrlKey || e.altKey) return;
      const view = crepe.editor.action((ctx) => ctx.get(editorViewCtx));
      if (!atStartOfBlockquote(view.state)) return;
      if (lift(view.state, view.dispatch)) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true,
  );
}
