/** Windows の「付箋」と同じ並びのアクセント色 (黄・緑・ピンク・紫・青・灰・チャコール) */
export const COLORS = ["#e6b905", "#6fd262", "#ea86c2", "#c78eff", "#5ac0e7", "#aaaaaa", "#616161"];

/** アクセント色の上に載せる文字・アイコンの色 */
export function foregroundOn(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
  return 0.299 * r + 0.587 * g + 0.114 * b < 128 ? "#f5f5f5" : "#1a1a1a";
}
