import { writable } from "svelte/store";
import type { Member, User } from "./types";

export interface ProfileTarget {
  user: User;
  member?: Member | null;
  anchor: HTMLElement | null;
  x: number;
  y: number;
}

export const profile = writable<ProfileTarget | null>(null);

export function openProfile(
  user: User,
  member: Member | null | undefined,
  anchor: HTMLElement | null,
) {
  const rect = anchor?.getBoundingClientRect();
  profile.set({
    user,
    member: member ?? null,
    anchor,
    x: rect?.left ?? 0,
    y: rect?.top ?? 0,
  });
}

export function closeProfile() {
  profile.set(null);
}
