import { emojiCategories, type EmojiEntry } from "./emoji-data";

export { emojiCategories };
export type { EmojiEntry };

const byChar = new Map<string, EmojiEntry>();
const byCharNoVariation = new Map<string, EmojiEntry>();
const assetFiles = new Set<string>();

function stripVariation(char: string): string {
  return char.replace(/[\uFE0E\uFE0F]/g, "");
}

for (const category of emojiCategories) {
  for (const entry of category.entries) {
    byChar.set(entry.char, entry);
    assetFiles.add(entry.file);
    const normalized = stripVariation(entry.char);
    if (normalized && !byCharNoVariation.has(normalized)) {
      byCharNoVariation.set(normalized, entry);
    }
  }
}

export function emojiCode(char: string): string {
  return [...char]
    .map((part) => part.codePointAt(0) ?? 0)
    .filter((code) => code !== 0xfe0f && code !== 0xfe0e)
    .map((code) => code.toString(16))
    .join("-");
}

export function emojiPath(char: string): string {
  const entry = byChar.get(char) ?? byCharNoVariation.get(stripVariation(char));
  if (entry?.file) return `/emoji/${entry.file}`;
  return `/emoji/${emojiCode(char)}.svg`;
}

export function isBundledEmoji(char: string): boolean {
  if (!char) return false;
  if (byChar.has(char)) return true;
  if (byCharNoVariation.has(stripVariation(char))) return true;
  return assetFiles.has(`${emojiCode(char)}.svg`);
}

const segmenter =
  typeof Intl !== "undefined" && "Segmenter" in Intl
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;

export type EmojiTextPart = { text: string } | { emoji: string };

export function splitEmoji(text: string): EmojiTextPart[] {
  const parts: EmojiTextPart[] = [];
  const pushText = (value: string) => {
    if (!value) return;
    const last = parts[parts.length - 1];
    if (last && "text" in last) last.text += value;
    else parts.push({ text: value });
  };
  if (!segmenter) {
    pushText(text);
    return parts;
  }
  for (const { segment } of segmenter.segment(text)) {
    if (isBundledEmoji(segment)) parts.push({ emoji: segment });
    else pushText(segment);
  }
  return parts;
}

export function emojiName(char: string): string {
  return byChar.get(char)?.name ?? char;
}

export function searchEmoji(query: string): EmojiEntry[] {
  const term = query.trim().toLowerCase();
  if (!term) return [];
  const seen = new Set<string>();
  const result: EmojiEntry[] = [];
  for (const category of emojiCategories) {
    for (const entry of category.entries) {
      if (seen.has(entry.char)) continue;
      if (
        entry.name.toLowerCase().includes(term) ||
        entry.keywords.toLowerCase().includes(term) ||
        entry.char === term
      ) {
        seen.add(entry.char);
        result.push(entry);
      }
    }
  }
  return result;
}

const RECENT_KEY = "flackle-recent-emoji";

export function getRecentEmoji(): string[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is string => typeof item === "string").slice(0, 20);
  } catch {
    return [];
  }
}

export function pushRecentEmoji(char: string) {
  const next = [char, ...getRecentEmoji().filter((item) => item !== char)].slice(0, 20);
  localStorage.setItem(RECENT_KEY, JSON.stringify(next));
}
