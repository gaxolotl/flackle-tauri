import { writable } from "svelte/store";
import type { User } from "./types";

export interface ConfirmRequest {
  title: string;
  body?: string;
  confirmLabel?: string;
  danger?: boolean;
  resolve: (value: boolean) => void;
}

export const confirmDialog = writable<ConfirmRequest | null>(null);

export function confirmAction(options: Omit<ConfirmRequest, "resolve">): Promise<boolean> {
  return new Promise((resolve) => {
    confirmDialog.set({ ...options, resolve });
  });
}

export interface ReportRequest {
  user: User;
  resolve: (value: { category: string; details: string } | null) => void;
}

export const reportDialog = writable<ReportRequest | null>(null);

export function reportAction(user: User): Promise<{ category: string; details: string } | null> {
  return new Promise((resolve) => {
    reportDialog.set({ user, resolve });
  });
}
