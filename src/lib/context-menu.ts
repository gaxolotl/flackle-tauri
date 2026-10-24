import { writable } from "svelte/store";

export interface ContextMenuItem {
  label: string;
  icon?: string;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  run?: () => void;
}

export interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
}

export const contextMenu = writable<ContextMenuState | null>(null);

export function openContextMenu(x: number, y: number, items: ContextMenuItem[]) {
  contextMenu.set({ x, y, items });
}

export function closeContextMenu() {
  contextMenu.set(null);
}
