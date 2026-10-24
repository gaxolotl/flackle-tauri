import { get } from "svelte/store";
import * as api from "./api";
import { openContextMenu, type ContextMenuItem } from "./context-menu";
import { confirmAction } from "./dialog";
import { closeProfile, openProfile } from "./profile";
import type { Member, User } from "./types";
import {
  acceptFriendRequest,
  blockUser,
  blockedUsers,
  cancelFriendRequest,
  friendRequests,
  friends,
  me,
  messageUser,
  modal,
  notify,
  removeFriend,
  sendFriendRequest,
  unblockUser,
} from "./workspace";

export interface UserMenuOptions {
  anchor?: HTMLElement | null;
  extraItems?: ContextMenuItem[];
}

export function showUserMenu(
  event: MouseEvent,
  user: User,
  member?: Member | null,
  options: UserMenuOptions = {},
) {
  event.preventDefault();
  event.stopPropagation();
  const anchor =
    options.anchor ?? (event.currentTarget as HTMLElement | null) ?? null;
  const self = get(me)?.id === user.id;
  const isFriend = get(friends).some((entry) => entry.id === user.id);
  const isBlocked = get(blockedUsers).some((entry) => entry.id === user.id);
  const incoming = get(friendRequests).incoming.find(
    (request) => request.sender?.id === user.id,
  );
  const outgoing = get(friendRequests).outgoing.find(
    (request) => request.receiver?.id === user.id,
  );
  const profileUrl = `https://flackle.com/u/${encodeURIComponent(user.username)}`;

  const run = (action: () => Promise<unknown>, success?: string) => {
    void action()
      .then(() => {
        if (success) notify(success);
      })
      .catch((error) =>
        notify(error instanceof Error ? error.message : String(error)),
      );
  };

  const items: ContextMenuItem[] = [
    {
      label: "Open Mini Profile",
      icon: "user",
      run: () => openProfile(user, member, anchor),
    },
    {
      label: "Open Full Profile",
      icon: "user",
      run: () => void api.openExternal(profileUrl),
    },
    {
      label: "Copy User URL",
      icon: "link",
      run: () => {
        void navigator.clipboard
          .writeText(profileUrl)
          .then(() => notify("Profile URL copied."))
          .catch(() => notify("Could not copy the profile URL."));
      },
    },
  ];

  if (!self) {
    items.push({
      label: "Message",
      icon: "mail",
      run: () => {
        void messageUser(user.id)
          .then(() => {
            modal.set(null);
          })
          .catch((error) =>
            notify(error instanceof Error ? error.message : String(error)),
          );
      },
    });

    if (isFriend) {
      items.push({
        label: "Remove Friend",
        icon: "user",
        danger: true,
        run: () => {
          void (async () => {
            const confirmed = await confirmAction({
              title: `Remove ${user.display_name || user.username}?`,
              body: "They will be removed from your friends list.",
              confirmLabel: "Remove friend",
              danger: true,
            });
            if (!confirmed) return;
            try {
              await removeFriend(user.id);
              notify("Friend removed.");
            } catch (error) {
              notify(error instanceof Error ? error.message : String(error));
            }
          })();
        },
      });
    } else if (incoming) {
      items.push({
        label: "Accept Friend Request",
        icon: "userPlus",
        run: () =>
          run(
            () => acceptFriendRequest(incoming.id),
            "Friend request accepted.",
          ),
      });
    } else if (outgoing) {
      items.push({
        label: "Cancel Friend Request",
        icon: "user",
        run: () =>
          run(
            () => cancelFriendRequest(outgoing.id),
            "Friend request cancelled.",
          ),
      });
    } else if (!isBlocked) {
      items.push({
        label: "Add Friend",
        icon: "userPlus",
        run: () =>
          run(() => sendFriendRequest(user.username), "Friend request sent."),
      });
    }

    items.push(
      isBlocked
        ? {
            label: "Unblock",
            icon: "ban",
            run: () => run(() => unblockUser(user.id), "User unblocked."),
          }
        : {
            label: "Block",
            icon: "ban",
            danger: true,
            run: () => {
              void (async () => {
                const confirmed = await confirmAction({
                  title: `Block ${user.display_name || user.username}?`,
                  body: "You will no longer see their messages or be able to message them.",
                  confirmLabel: "Block user",
                  danger: true,
                });
                if (!confirmed) return;
                try {
                  await blockUser(user.id);
                  closeProfile();
                  notify("User blocked.");
                } catch (error) {
                  notify(error instanceof Error ? error.message : String(error));
                }
              })();
            },
          },
    );
  }

  items.push(
    { separator: true, label: "" },
    {
      label: "Copy ID",
      icon: "copy",
      run: () => {
        void navigator.clipboard
          .writeText(user.id)
          .then(() => notify("User ID copied."))
          .catch(() => notify("Could not copy the user ID."));
      },
    },
    {
      label: "Translate",
      icon: "translate",
      run: () => {
        void api.openExternal(
          `https://translate.google.com/translate?sl=auto&tl=en&u=${encodeURIComponent(profileUrl)}`,
        );
      },
    },
  );

  if (options.extraItems?.length) {
    items.push({ separator: true, label: "" }, ...options.extraItems);
  }

  openContextMenu(event.clientX, event.clientY, items);
}
