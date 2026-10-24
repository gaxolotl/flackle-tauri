import { writable } from "svelte/store";
import type { Category, Channel } from "./types";

export type ChannelDialogMode =
  | "channel-create"
  | "channel-edit"
  | "category-create"
  | "category-edit";

export interface ChannelDialogRequest {
  mode: ChannelDialogMode;
  serverId: string;
  serverName: string;
  channel?: Channel | null;
  category?: Category | null;
  categoryId?: string | null;
}

export const channelDialog = writable<ChannelDialogRequest | null>(null);

export function openChannelCreate(
  serverId: string,
  serverName: string,
  categoryId: string | null = null,
) {
  channelDialog.set({
    mode: "channel-create",
    serverId,
    serverName,
    categoryId,
  });
}

export function openChannelEdit(serverId: string, serverName: string, channel: Channel) {
  channelDialog.set({
    mode: "channel-edit",
    serverId,
    serverName,
    channel,
    categoryId: channel.category_id ?? null,
  });
}

export function openCategoryCreate(serverId: string, serverName: string) {
  channelDialog.set({
    mode: "category-create",
    serverId,
    serverName,
  });
}

export function openCategoryEdit(serverId: string, serverName: string, category: Category) {
  channelDialog.set({
    mode: "category-edit",
    serverId,
    serverName,
    category,
  });
}

export function closeChannelDialog() {
  channelDialog.set(null);
}
