import type {
  Bootstrap,
  ChannelBundle,
  Member,
  MemberListSection,
  MemberListSnapshot,
  Message,
  MessagePage,
  Role,
  SelfUser,
  Target,
  User,
} from "./types";

const now = Date.now();
const at = (minutesAgo: number) => new Date(now - minutesAgo * 60_000).toISOString();

export const me: SelfUser = {
  id: "100000000000000001",
  username: "gosho",
  display_name: "Gosho",
  email: "gosho@example.com",
  is_admin: false,
  status: "online",
  banner_url: "/brand/flackle-backdrop.svg",
};

const gax: User = {
  id: "100000000000000002",
  username: "gax",
  display_name: "gax",
  status: "online",
};

const mira: User = {
  id: "100000000000000003",
  username: "mira",
  display_name: "Mira Chen",
  status: "idle",
  banner_url: "/demo/server-garage.svg",
};

const jun: User = {
  id: "100000000000000004",
  username: "jun",
  display_name: "jun",
  status: "dnd",
};

const sol: User = {
  id: "100000000000000005",
  username: "sol",
  display_name: "Sol",
  status: "online",
};

const riley: User = {
  id: "100000000000000006",
  username: "riley",
  display_name: "Riley",
  status: "offline",
};

export const demoRoles: Role[] = [
  {
    id: "700000000000000000",
    name: "@everyone",
    permissions: "0",
    position: 0,
    color: null,
    is_everyone: true,
  },
  {
    id: "700000000000000001",
    name: "Founder",
    permissions: "0",
    position: 100,
    color: "#e0a13d",
    hoist: true,
    member_count: 1,
  },
  {
    id: "700000000000000002",
    name: "Design",
    permissions: "0",
    position: 80,
    color: "#8b6ce8",
    hoist: true,
    member_count: 2,
  },
  {
    id: "700000000000000003",
    name: "Engineer",
    permissions: "0",
    position: 70,
    color: "#2fbd9f",
    hoist: true,
    member_count: 1,
  },
  {
    id: "700000000000000004",
    name: "Member",
    permissions: "0",
    position: 10,
    color: "#7b8794",
    member_count: 1,
  },
];

const serverSeeds = [
  {
    id: "200000000000000001",
    name: "Flackle HQ",
    initials: "FH",
    color: "#ef5252",
    icon: "/demo/server-hq.svg",
    description: "The unofficial home for the desktop build.",
    channels: [
      { id: "300000000000000001", name: "welcome", topic: "Start here. Say hi.", icon: "ChatBubbleLeftIcon", icon_color: "#95a5a6" },
      { id: "300000000000000002", name: "general", topic: "Anything and everything.", icon: "ChatBubbleLeftIcon", icon_color: "#95a5a6" },
      { id: "300000000000000003", name: "dev-log", topic: "What we shipped today.", icon: "CodeBracketIcon", icon_color: "#95a5a6" },
      { id: "300000000000000004", name: "design", topic: "Screens, systems, and nice corners.", icon: "PaintBrushIcon", icon_color: "#8b6ce8" },
      { id: "300000000000000005", name: "show-and-tell", topic: "Show us what you made.", icon: "CameraIcon", icon_color: "#546e7a" },
    ],
  },
  {
    id: "200000000000000002",
    name: "Night Shift",
    initials: "NS",
    color: "#8b6ce8",
    icon: "/demo/server-night.svg",
    description: "Late crew, long builds, quiet channels.",
    channels: [
      { id: "310000000000000001", name: "lobby", topic: "Late crew only.", icon: "ChatBubbleLeftRightIcon", icon_color: "#8b6ce8" },
      { id: "310000000000000002", name: "music", topic: "What's on loop?", icon: "MusicalNoteIcon", icon_color: "#e05ea8" },
      { id: "310000000000000003", name: "screenshots", topic: "No context needed.", icon: "CameraIcon", icon_color: "#546e7a" },
    ],
  },
  {
    id: "200000000000000003",
    name: "Design Garage",
    initials: "DG",
    color: "#2fbd9f",
    icon: "/demo/server-garage.svg",
    description: "Critique, type, and the corners nobody notices.",
    channels: [
      { id: "320000000000000001", name: "critique", topic: "Be specific, be kind.", icon: "PaintBrushIcon", icon_color: "#2fbd9f" },
      { id: "320000000000000002", name: "type", topic: "Kerning crimes go here.", icon: "DocumentTextIcon", icon_color: "#607d8b" },
    ],
  },
  {
    id: "200000000000000004",
    name: "Book Club",
    initials: "BC",
    color: "#e0a13d",
    icon: "/demo/server-books.svg",
    description: "One book at a time. The pile still grows.",
    channels: [
      { id: "330000000000000001", name: "current-read", topic: "Piranesi, chapters 1–12.", icon: "BookOpenIcon", icon_color: "#e0a13d" },
      { id: "330000000000000002", name: "recommendations", topic: "The pile grows.", icon: "SparklesIcon", icon_color: "#e0a13d" },
    ],
  },
];

export const demoBootstrap: Bootstrap = {
  me,
  servers: serverSeeds.map((server, index) => ({
    id: server.id,
    name: server.name,
    description: server.description,
    icon_url: server.icon,
    banner_url: index === 0 ? "/brand/flackle-backdrop.svg" : null,
    default_notification_level: "mentions",
    created_at: at(500_000),
    position: index,
    approx_member_count: [128, 42, 19, 76][index],
    approx_presence_count: [37, 12, 5, 14][index],
  })),
  conversations: [
    {
      id: "400000000000000001",
      type: "dm",
      last_message_at: at(4),
      unread_count: 2,
      members: [{ user: gax }],
      last_message: {
        id: "m-dm-1",
        content: "pushed the theme pass — tell me what you think",
        type: "text",
        is_deleted: false,
        created_at: at(4),
        updated_at: at(4),
        sender: gax,
      },
    },
    {
      id: "400000000000000002",
      type: "group",
      name: "design crew",
      last_message_at: at(38),
      members: [{ user: mira }, { user: jun }],
      last_message: {
        id: "m-dm-2",
        content: "the new rail spacing feels right",
        type: "text",
        is_deleted: false,
        created_at: at(38),
        updated_at: at(38),
        sender: mira,
      },
    },
    {
      id: "400000000000000003",
      type: "dm",
      last_message_at: at(190),
      members: [{ user: sol }],
      last_message: {
        id: "m-dm-3",
        content: "sent you the invite",
        type: "text",
        is_deleted: false,
        created_at: at(190),
        updated_at: at(190),
        sender: sol,
      },
    },
  ],
  unread: {
    channels: [
      {
        server_id: "200000000000000001",
        channel_id: "300000000000000004",
        unread_count: 3,
        mention_count: 1,
      },
      {
        server_id: "200000000000000001",
        channel_id: "300000000000000003",
        unread_count: 1,
        mention_count: 0,
      },
      {
        server_id: "200000000000000003",
        channel_id: "320000000000000001",
        unread_count: 7,
        mention_count: 0,
      },
    ],
  },
};

export function demoChannels(serverId: string): ChannelBundle {
  const server = serverSeeds.find((item) => item.id === serverId) ?? serverSeeds[0];
  const cut = Math.ceil(server.channels.length / 2);
  const first = server.channels.slice(0, cut);
  const second = server.channels.slice(cut);
  return {
    categories: [
      { id: "cat-1", name: "Lounge", position: 0 },
      { id: "cat-2", name: "Projects", position: 1 },
    ],
    channels: [
      ...first.map((channel, index) => ({
        id: channel.id,
        name: channel.name,
        topic: channel.topic,
        icon: channel.icon,
        icon_color: channel.icon_color,
        type: "text",
        position: index,
        category_id: "cat-1",
      })),
      ...second.map((channel, index) => ({
        id: channel.id,
        name: channel.name,
        topic: channel.topic,
        icon: channel.icon,
        icon_color: channel.icon_color,
        type: "text",
        position: index,
        category_id: "cat-2",
      })),
    ],
    permissions: {},
  };
}

const generalSeed: Array<[User, string, number, string?]> = [
  [gax, "morning. the desktop build finally connects to the gateway without the browser shim", 186],
  [gax, "which means we can ship a real client instead of wrapping the web app", 185],
  [mira, "huge. what's the install size looking like?", 180],
  [gax, "roughly 7 MB on windows, most of it is the rust + sqlite bits. webview is the system one", 179],
  [jun, "that's absurdly small for a chat app", 176],
  [mira, "and it still looks like this? ok", 175],
  [gax, "wait until you see the command palette", 174],
  [sol, "the red accent is doing a lot of work. in a good way 🔥", 162],
  [mira, "one note: message hover actions should fade in, not pop", 140],
  [gax, "agreed. 120ms, only on pointer devices", 138],
  [jun, "can we keep the member list collapsible? narrow windows get tight", 61],
  [gax, "already in. cmd+shift+m or the little people icon", 58],
  [mira, "typing indicator feels human now", 12],
  [sol, "ship it __tonight__ 🚀", 8],
];

export function demoMessages(target: Target, before?: string | null): MessagePage {
  if ("conversation_id" in target) {
    const conversation = demoBootstrap.conversations.find(
      (item) => item.id === target.conversation_id,
    );
    const person = conversation?.members?.[0]?.user ?? gax;
    const messages: Message[] = [
      {
        id: `${target.conversation_id}-1`,
        content:
          conversation?.type === "group"
            ? "the new rail spacing feels right. the icons finally breathe"
            : "pushed the theme pass — tell me what you think",
        type: "text",
        is_deleted: false,
        created_at: at(44),
        updated_at: at(44),
        sender: person,
      },
      {
        id: `${target.conversation_id}-2`,
        content:
          "the contrast on muted text is better too. my eyes stopped filing a complaint",
        type: "text",
        is_deleted: false,
        created_at: at(41),
        updated_at: at(41),
        sender: me,
      },
      {
        id: `${target.conversation_id}-3`,
        content: "good. next up: attachments and then a real search",
        type: "text",
        is_deleted: false,
        created_at: at(9),
        updated_at: at(9),
        sender: person,
        reactions: [{ emoji: "🔥", count: 2, me: true, users: [me, person] }],
      },
    ];
    return {
      messages: before ? [] : messages,
      has_more: false,
      has_more_before: false,
      has_more_after: false,
    };
  }

  const channelId = target.channel_id;
  const server = serverSeeds.find((item) =>
    item.channels.some((channel) => channel.id === channelId),
  );
  const channel = server?.channels.find((item) => item.id === channelId);
  if (!channel) {
    return { messages: [], has_more: false, has_more_before: false, has_more_after: false };
  }

  const messages: Message[] = generalSeed.map(([sender, content, minutes, reaction], index) => ({
    id: `${channelId}${String(index).padStart(6, "0")}`,
    content,
    type: "text",
    is_deleted: false,
    created_at: at(minutes),
    updated_at: at(minutes),
    sender,
    reactions: reaction
      ? [{ emoji: reaction, count: 3, me: false, users: [mira, jun, sol] }]
      : [],
  }));

  if (channel.name === "welcome") {
    messages[2].reactions = [
      { emoji: "🔥", count: 4, me: false, users: [gax, jun, sol, mira] },
    ];
    messages[3].reactions = [
      { emoji: "👍", count: 3, me: true, users: [me, jun, sol] },
      { emoji: "😂", count: 2, me: false, users: [mira, jun] },
    ];

    messages.splice(
      10,
      0,
      {
        id: `${channelId}900001`,
        content:
          "shipped the first Windows build tonight. single 7 MB exe, no bundled runtime, starts in under a second.",
        type: "text",
        is_deleted: false,
        created_at: at(130),
        updated_at: at(130),
        sender: me,
        reactions: [
          { emoji: "🚀", count: 5, me: true, users: [me, gax, mira, jun, sol] },
          { emoji: "🎉", count: 2, me: false, users: [mira, sol] },
        ],
        attachments: [
          {
            url: "",
            file_name: "flackle-1.0.0-notes.md",
            file_size: 8_420,
            content_type: "text/markdown",
          },
        ],
      },
      {
        id: `${channelId}900002`,
        content: "release notes and checksums are in there. macOS universal is next.",
        type: "text",
        is_deleted: false,
        created_at: at(128),
        updated_at: at(128),
        sender: me,
      },
    );

    messages.push(
      {
        id: `${channelId}900003`,
        content: `<@&700000000000000002> the new <#300000000000000004> screens are up. <@100000000000000003> left the context here: https://flackle.com/app/channels/200000000000000001/300000000000000001#message-${messages[3].id}`,
        type: "text",
        is_deleted: false,
        created_at: at(7),
        updated_at: at(7),
        sender: mira,
        reactions: [{ emoji: "👀", count: 3, me: false, users: [gax, jun, sol] }],
      },
      {
        id: `${channelId}900004`,
        content: "installed it on the couch laptop. starts before the fan spins up",
        type: "text",
        is_deleted: false,
        created_at: at(6),
        updated_at: at(6),
        sender: sol,
        reply_to: {
          id: messages[3].id,
          content: messages[3].content,
          sender_id: gax.id,
          sender_username: gax.username,
          sender_display_name: gax.display_name ?? gax.username,
        },
      },
    );
  }

  if (channel.name === "general") {
    messages.splice(7, 2,
      {
        id: `${channelId}900101`,
        content: "Gosho pinned a message to this channel.",
        type: "system",
        is_deleted: false,
        created_at: at(170),
        updated_at: at(170),
      },
      {
        id: `${channelId}900102`,
        content: "pulled the palette from the new mark — four values, one accent",
        type: "text",
        is_deleted: false,
        created_at: at(168),
        updated_at: at(168),
        sender: mira,
      },
    );
  }

  return {
    messages: before ? messages.slice(0, 5) : messages,
    has_more: false,
    has_more_before: Boolean(before),
    has_more_after: false,
  };
}

export const demoMembers: Member[] = [
  { user: me, nickname: me.display_name ?? null, is_owner: false, role_ids: ["700000000000000002"], presence: "online", joined_at: at(220_000) },
  { user: gax, nickname: gax.display_name ?? null, is_owner: true, role_ids: ["700000000000000001"], presence: "online", joined_at: at(400_000) },
  { user: mira, nickname: mira.display_name ?? null, is_owner: false, role_ids: ["700000000000000002"], presence: "idle", joined_at: at(180_000) },
  { user: jun, nickname: jun.display_name ?? null, is_owner: false, role_ids: ["700000000000000003"], presence: "dnd", joined_at: at(120_000) },
  { user: sol, nickname: sol.display_name ?? null, is_owner: false, role_ids: ["700000000000000004"], presence: "online", joined_at: at(60_000) },
  { user: riley, nickname: riley.display_name ?? null, is_owner: false, role_ids: ["700000000000000004"], presence: "offline", joined_at: at(30_000) },
];

export function demoMemberSnapshot(): MemberListSnapshot {
  const roleMemberIds = new Set<string>();
  const sections: MemberListSection[] = demoRoles
    .filter((role) => role.hoist)
    .map((role) => {
      const ids = demoMembers
        .filter((member) => (member.role_ids ?? []).includes(role.id))
        .map((member) => member.user.id);
      ids.forEach((id) => roleMemberIds.add(id));
      return {
        key: `role:${role.id}`,
        label: role.name,
        role_id: role.id,
        count: ids.length,
        member_ids: ids,
      };
    })
    .filter((section) => section.count > 0);
  const remaining = demoMembers.filter((member) => !roleMemberIds.has(member.user.id));
  const online = remaining.filter((member) =>
    ["online", "idle", "dnd"].includes(member.presence ?? "offline"),
  );
  const offline = remaining.filter(
    (member) => !["online", "idle", "dnd"].includes(member.presence ?? "offline"),
  );
  if (online.length) {
    sections.push({
      key: "online",
      label: "Online",
      role_id: null,
      count: online.length,
      member_ids: online.map((member) => member.user.id),
    });
  }
  if (offline.length) {
    sections.push({
      key: "offline",
      label: "Offline",
      role_id: null,
      count: offline.length,
      member_ids: offline.map((member) => member.user.id),
    });
  }
  return {
    members: demoMembers,
    total: demoMembers.length,
    sections,
    roles: demoRoles,
  };
}
