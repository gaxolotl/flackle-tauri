import { get, writable } from "svelte/store";

export type Theme = "dark" | "light" | "system";

const KEY = "flackle-theme";

export function getTheme(): Theme {
  const value = localStorage.getItem(KEY);
  if (value === "light" || value === "system") return value;
  return "dark";
}

export function effectiveTheme(theme: Theme): "dark" | "light" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  return theme;
}

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = effectiveTheme(theme);
}

export const theme = writable<Theme>(getTheme());

export function setTheme(next: Theme) {
  localStorage.setItem(KEY, next);
  theme.set(next);
  applyTheme(next);
}

// Keep system mode in sync without a reload.
if (typeof window !== "undefined") {
  const media = window.matchMedia("(prefers-color-scheme: light)");
  media.addEventListener("change", () => {
    if (get(theme) === "system") applyTheme("system");
  });
}
