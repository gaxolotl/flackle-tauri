import { writable } from "svelte/store";

const STORAGE_KEY = "flackle-stats-elevated";

function initialVisibility(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== "0";
  } catch {
    return true;
  }
}

export const statsElevatedVisible = writable(initialVisibility());

statsElevatedVisible.subscribe((visible) => {
  try {
    localStorage.setItem(STORAGE_KEY, visible ? "1" : "0");
  } catch {
    // The preference is optional; keep the in-memory value.
  }
});
