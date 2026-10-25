import { writable } from "svelte/store";

export interface ImageView {
  url: string;
  alt: string;
}

export const imageViewer = writable<ImageView | null>(null);

export function openImage(url: string | null | undefined, alt = "Image") {
  if (!url) return;
  imageViewer.set({ url, alt });
}

export function closeImage() {
  imageViewer.set(null);
}
