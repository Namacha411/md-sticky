import { Crepe } from "@milkdown/crepe";
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
  return { flush, discard };
}
