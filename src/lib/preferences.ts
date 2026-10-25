import { get, writable } from "svelte/store";

export type AccentName = "ember" | "violet" | "teal" | "blue" | "gold";
export type Density = "cozy" | "compact";
export type TextSize = "small" | "default" | "large";
export type TimeFormat = "12h" | "24h";
export type SendKey = "enter" | "mod-enter";

export type KeybindAction =
  | "quickSwitcher"
  | "settings"
  | "toggleMembers"
  | "serverSettings";

export type Keybinds = Record<KeybindAction, string>;

export interface KeybindDefinition {
  id: KeybindAction;
  label: string;
  description: string;
  defaultValue: string;
}

export const keybindDefinitions: KeybindDefinition[] = [
  {
    id: "quickSwitcher",
    label: "Quick switcher",
    description: "Jump to a server, channel, or conversation.",
    defaultValue: "Mod+K",
  },
  {
    id: "settings",
    label: "Open settings",
    description: "Open the app settings panel.",
    defaultValue: "Mod+,",
  },
  {
    id: "toggleMembers",
    label: "Toggle member list",
    description: "Show or hide the server member roster.",
    defaultValue: "Mod+Shift+M",
  },
  {
    id: "serverSettings",
    label: "Server settings",
    description: "Open settings for the server you are viewing.",
    defaultValue: "Mod+Alt+S",
  },
];

export const defaultKeybinds: Keybinds = {
  quickSwitcher: "Mod+K",
  settings: "Mod+,",
  toggleMembers: "Mod+Shift+M",
  serverSettings: "Mod+Alt+S",
};

export interface Preferences {
  accent: AccentName;
  density: Density;
  textSize: TextSize;
  reduceMotion: boolean;
  startWithMembers: boolean;
  notifications: boolean;
  mentionsOnly: boolean;
  timeFormat: TimeFormat;
  sendKey: SendKey;
  showTyping: boolean;
  shareTyping: boolean;
  linkPreviews: boolean;
  characterCounter: boolean;
  mentionAvatars: boolean;
  messageClickActions: boolean;
  quickReactions: boolean;
  hidePendingCount: boolean;
  noTypingAnimation: boolean;
  onePingPerDm: boolean;
  pictureInPicture: boolean;
  previewMessage: boolean;
  quickDelete: boolean;
  showMeYourName: boolean;
  whoReacted: boolean;
  keepCurrentChannel: boolean;
  viewIcons: boolean;
  showMediaExpiry: boolean;
  keybinds: Keybinds;
}

export type EnhancementKey =
  | "linkPreviews"
  | "characterCounter"
  | "mentionAvatars"
  | "messageClickActions"
  | "quickReactions"
  | "hidePendingCount"
  | "noTypingAnimation"
  | "onePingPerDm"
  | "pictureInPicture"
  | "previewMessage"
  | "quickDelete"
  | "showMeYourName"
  | "whoReacted"
  | "keepCurrentChannel"
  | "viewIcons"
  | "showMediaExpiry";

export interface EnhancementDefinition {
  key: EnhancementKey;
  label: string;
  description: string;
}

export const enhancementDefinitions: EnhancementDefinition[] = [
  {
    key: "linkPreviews",
    label: "Link previews",
    description: "Fetch and render rich previews for links in messages.",
  },
  {
    key: "showMediaExpiry",
    label: "Attachment expiry",
    description: "Show when an uploaded attachment will expire, like Flackle does.",
  },
  {
    key: "characterCounter",
    label: "Character counter",
    description: "Show the character count and the message limit under the composer.",
  },
  {
    key: "mentionAvatars",
    label: "Mention avatars",
    description: "Show the avatar next to user mentions in messages.",
  },
  {
    key: "messageClickActions",
    label: "Message click actions",
    description: "Double-click a message to reply to it.",
  },
  {
    key: "quickReactions",
    label: "Quick reactions",
    description: "Add your recently used emoji to the message action bar.",
  },
  {
    key: "hidePendingCount",
    label: "Hide pending count",
    description: "Hide the friend-request badge on the Direct Messages button.",
  },
  {
    key: "noTypingAnimation",
    label: "No typing animation",
    description: "Stop the typing dots from bouncing.",
  },
  {
    key: "onePingPerDm",
    label: "One ping per DM",
    description: "Only notify once per conversation until you read it.",
  },
  {
    key: "pictureInPicture",
    label: "Picture in picture",
    description: "Add a picture-in-picture control to inline videos.",
  },
  {
    key: "previewMessage",
    label: "Preview message",
    description: "Preview formatting, mentions, and emoji below the composer.",
  },
  {
    key: "quickDelete",
    label: "Quick delete",
    description: "Delete your own messages immediately without a confirmation dialog.",
  },
  {
    key: "showMeYourName",
    label: "Show me your name",
    description: "Show usernames next to display names in messages and the member list.",
  },
  {
    key: "whoReacted",
    label: "Who reacted",
    description: "List the people behind each reaction in its tooltip.",
  },
  {
    key: "keepCurrentChannel",
    label: "Keep current channel",
    description: "Reopen the channel or conversation you were viewing next launch.",
  },
  {
    key: "viewIcons",
    label: "View icons",
    description: "Add buttons to view a profile's avatar and banner in the app.",
  },
];

export const accentOptions: Array<{
  id: AccentName;
  label: string;
  rgb: string;
}> = [
  { id: "ember", label: "Ember", rgb: "239, 82, 82" },
  { id: "violet", label: "Violet", rgb: "139, 108, 232" },
  { id: "teal", label: "Teal", rgb: "47, 189, 159" },
  { id: "blue", label: "Blue", rgb: "74, 155, 232" },
  { id: "gold", label: "Gold", rgb: "224, 161, 61" },
];

const KEY = "flackle-preferences";

const defaults: Preferences = {
  accent: "ember",
  density: "cozy",
  textSize: "default",
  reduceMotion: false,
  startWithMembers: true,
  notifications: true,
  mentionsOnly: false,
  timeFormat: "12h",
  sendKey: "enter",
  showTyping: true,
  shareTyping: true,
  linkPreviews: true,
  characterCounter: true,
  mentionAvatars: true,
  messageClickActions: false,
  quickReactions: true,
  hidePendingCount: false,
  noTypingAnimation: false,
  onePingPerDm: false,
  pictureInPicture: true,
  previewMessage: false,
  quickDelete: false,
  showMeYourName: false,
  whoReacted: true,
  keepCurrentChannel: false,
  viewIcons: true,
  showMediaExpiry: true,
  keybinds: { ...defaultKeybinds },
};

function load(): Preferences {
  try {
    const parsed = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Preferences>;
    return {
      ...defaults,
      ...parsed,
      accent: accentOptions.some((option) => option.id === parsed.accent)
        ? (parsed.accent as AccentName)
        : defaults.accent,
      keybinds: {
        ...defaultKeybinds,
        ...(parsed.keybinds ?? {}),
      },
    };
  } catch {
    return { ...defaults, keybinds: { ...defaultKeybinds } };
  }
}

export function applyPreferences(value: Preferences) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.density = value.density;
  root.dataset.textSize = value.textSize;
  root.dataset.reduceMotion = value.reduceMotion ? "true" : "false";
  const accent = accentOptions.find((option) => option.id === value.accent) ?? accentOptions[0];
  root.style.setProperty("--accent-rgb", accent.rgb);
  root.style.setProperty("--accent", `rgb(${accent.rgb})`);
  root.style.setProperty("--accent-dim", `rgba(${accent.rgb}, 0.12)`);
}

export const prefs = writable<Preferences>(load());

prefs.subscribe((value) => {
  try {
    localStorage.setItem(KEY, JSON.stringify(value));
  } catch {
    // Storage can be unavailable in private windows; preferences stay in memory.
  }
  applyPreferences(value);
});

export function setPreference<K extends keyof Preferences>(key: K, value: Preferences[K]) {
  prefs.update((current) => ({ ...current, [key]: value }));
}

export function accentRgb(name: AccentName): string {
  return (accentOptions.find((option) => option.id === name) ?? accentOptions[0]).rgb;
}

export function resetPreferences() {
  prefs.set({ ...defaults, keybinds: { ...defaultKeybinds } });
}

export function setKeybind(action: KeybindAction, binding: string) {
  prefs.update((current) => ({
    ...current,
    keybinds: { ...current.keybinds, [action]: binding },
  }));
}

export function resetKeybind(action: KeybindAction) {
  setKeybind(action, defaultKeybinds[action]);
}

export function resetAllKeybinds() {
  prefs.update((current) => ({
    ...current,
    keybinds: { ...defaultKeybinds },
  }));
}

function isMacPlatform() {
  return typeof navigator !== "undefined" && /Mac|iPhone|iPad|iPod/i.test(navigator.platform);
}

function normalizeKey(key: string): string {
  if (key === " " || key === "Spacebar") return "Space";
  if (key === "Esc") return "Escape";
  if (key === "+") return "Plus";
  if (key === "-") return "Minus";
  if (key.length === 1) return key.toUpperCase();
  return key;
}

function isModifierKey(key: string) {
  return ["Shift", "Control", "Alt", "Meta", "CapsLock", "OS"].includes(key);
}

export function keybindFromEvent(event: KeyboardEvent): string | null {
  if (isModifierKey(event.key)) return null;
  if (event.key === "Escape") return null;
  const parts: string[] = [];
  if (event.metaKey || event.ctrlKey) parts.push("Mod");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  parts.push(normalizeKey(event.key));
  return parts.join("+");
}

export function matchesKeybind(event: KeyboardEvent, binding: string | null | undefined): boolean {
  if (!binding) return false;
  const tokens = binding
    .split("+")
    .map((token) => token.trim().toLowerCase())
    .filter(Boolean);
  const keyToken = tokens.find(
    (token) => !["mod", "ctrl", "control", "meta", "cmd", "command", "shift", "alt", "option"].includes(token),
  );
  if (!keyToken) return false;
  const needsMod = tokens.some((token) =>
    ["mod", "ctrl", "control", "meta", "cmd", "command"].includes(token),
  );
  const needsShift = tokens.includes("shift");
  const needsAlt = tokens.some((token) => token === "alt" || token === "option");
  const hasMod = event.metaKey || event.ctrlKey;
  if (needsMod !== hasMod) return false;
  if (needsShift !== event.shiftKey) return false;
  if (needsAlt !== event.altKey) return false;
  return normalizeKey(event.key).toLowerCase() === keyToken;
}

export function formatKeybind(binding: string): string {
  if (!binding) return "Unassigned";
  const mac = isMacPlatform();
  return binding
    .split("+")
    .map((token) => {
      switch (token.toLowerCase()) {
        case "mod":
          return mac ? "⌘" : "Ctrl";
        case "shift":
          return mac ? "⇧" : "Shift";
        case "alt":
        case "option":
          return mac ? "⌥" : "Alt";
        case "space":
          return "Space";
        case "plus":
          return "+";
        case "minus":
          return "-";
        case "escape":
          return "Esc";
        default:
          return token.length === 1 ? token.toUpperCase() : token;
      }
    })
    .join(" + ");
}

export function keybindTokens(binding: string): string[] {
  return formatKeybind(binding)
    .split(" + ")
    .filter(Boolean);
}

export function getTimeFormat(): TimeFormat {
  return get(prefs).timeFormat;
}

export function getSendKey(): SendKey {
  return get(prefs).sendKey;
}

export function getNotificationPreferences() {
  const value = get(prefs);
  return { enabled: value.notifications, mentionsOnly: value.mentionsOnly };
}
