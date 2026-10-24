use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub type Snowflake = String;

fn default_kind() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct User {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub is_online: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl User {
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfUser {
    #[serde(flatten)]
    pub user: User,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub membership: Value,
    #[serde(default)]
    pub limits: Value,
}

impl std::ops::Deref for SelfUser {
    type Target = User;

    fn deref(&self) -> &User {
        &self.user
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Attachment {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageReply {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub sender_id: Snowflake,
    #[serde(default)]
    pub sender_username: Option<String>,
    #[serde(default)]
    pub sender_display_name: Option<String>,
    #[serde(default)]
    pub sender_avatar_url: Option<String>,
    #[serde(default)]
    pub has_attachments: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Reaction {
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub me: bool,
    #[serde(default)]
    pub users: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub conversation_id: Option<Snowflake>,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "type", default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub edited_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub sender: Option<User>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Value>,
    #[serde(default)]
    pub reply_to: Option<MessageReply>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub mention_ids: Vec<Snowflake>,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Server {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub owner_id: Option<Snowflake>,
    #[serde(default)]
    pub default_notification_level: Option<String>,
    #[serde(default)]
    pub visibility_status: Option<String>,
    #[serde(default)]
    pub approx_member_count: Option<u32>,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Channel {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(rename = "type", default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub category_id: Option<Snowflake>,
    #[serde(default)]
    pub slowmode_seconds: Option<u32>,
    #[serde(default)]
    pub permission_synced: Option<bool>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_color: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Category {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelBundle {
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub permissions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationMember {
    #[serde(default)]
    pub user: User,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conversation {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(rename = "type", default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub is_muted: bool,
    #[serde(default)]
    pub last_message_at: Option<String>,
    #[serde(default)]
    pub last_message: Option<Message>,
    #[serde(default)]
    pub members: Vec<ConversationMember>,
    #[serde(default)]
    pub unread_count: u32,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Conversation {
    pub fn title(&self, me: Option<&SelfUser>) -> String {
        if let Some(name) = &self.name {
            if !name.is_empty() {
                return name.clone();
            }
        }
        let others: Vec<&str> = self
            .members
            .iter()
            .filter(|m| me.map(|u| m.user.id != u.id).unwrap_or(true))
            .map(|m| m.user.name())
            .collect();
        if others.is_empty() {
            "Direct message".to_string()
        } else {
            others.join(", ")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Member {
    #[serde(default)]
    pub user: User,
    #[serde(default)]
    pub user_id: Option<Snowflake>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<Snowflake>,
    #[serde(default)]
    pub roles: Vec<Snowflake>,
    #[serde(default)]
    pub presence: Option<String>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub timed_out_until: Option<String>,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Member {
    pub fn id(&self) -> &str {
        self.user_id.as_deref().unwrap_or(&self.user.id)
    }

    pub fn name(&self) -> &str {
        self.nickname.as_deref().unwrap_or_else(|| self.user.name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemberPage {
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Role {
    #[serde(default)]
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub permissions: String,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub hoist: bool,
    #[serde(default)]
    pub mentionable: bool,
    #[serde(default)]
    pub is_everyone: bool,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub icon_emoji: Option<String>,
    #[serde(default)]
    pub member_count: Option<u32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RolesResponse {
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub can_view_role_permissions: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemberListSection {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub role_id: Option<Snowflake>,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub member_ids: Vec<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemberListSnapshot {
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub sections: Vec<MemberListSection>,
    #[serde(default)]
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcceptInviteResult {
    #[serde(default)]
    pub server: Option<Server>,
    #[serde(default)]
    pub landing_channel_id: Option<Snowflake>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerUserSettings {
    #[serde(default)]
    pub server_id: Snowflake,
    #[serde(default)]
    pub notification_level: Option<String>,
    #[serde(default)]
    pub last_viewed_channel_id: Option<Snowflake>,
    #[serde(default)]
    pub collapsed_categories: Vec<Snowflake>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerSettingsBundle {
    #[serde(default)]
    pub server_settings: Vec<ServerUserSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateServerResult {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub channel: Channel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Invite {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub server_id: Option<Snowflake>,
    #[serde(default)]
    pub channel_id: Option<Snowflake>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionResponse {
    #[serde(default)]
    pub permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_notification_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowmode_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateChannel {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowmode_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagePage {
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub has_more_before: bool,
    #[serde(default)]
    pub has_more_after: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnreadChannel {
    #[serde(default)]
    pub server_id: Snowflake,
    #[serde(default)]
    pub channel_id: Snowflake,
    #[serde(default)]
    pub unread_count: u32,
    #[serde(default)]
    pub mention_count: u32,
    #[serde(default)]
    pub blocked_unread_count: u32,
    #[serde(default)]
    pub blocked_mention_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnreadSummary {
    #[serde(default)]
    pub channels: Vec<UnreadChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    Ready {
        session_id: String,
        user: Option<SelfUser>,
        presences: HashMap<Snowflake, String>,
    },
    Connected,
    Disconnected {
        code: u16,
        reason: String,
    },
    MessageCreate {
        message: Message,
        server_id: Option<Snowflake>,
        channel_id: Option<Snowflake>,
        conversation_id: Option<Snowflake>,
    },
    MessageUpdate {
        id: Snowflake,
        content: String,
        edited_at: Option<String>,
        server_id: Option<Snowflake>,
        channel_id: Option<Snowflake>,
        conversation_id: Option<Snowflake>,
    },
    MessageDelete {
        id: Snowflake,
        server_id: Option<Snowflake>,
        channel_id: Option<Snowflake>,
        conversation_id: Option<Snowflake>,
    },
    MessagesBulkDelete {
        ids: Vec<Snowflake>,
        server_id: Snowflake,
        channel_id: Snowflake,
    },
    Typing {
        user_id: Snowflake,
        is_typing: bool,
        server_id: Option<Snowflake>,
        channel_id: Option<Snowflake>,
        conversation_id: Option<Snowflake>,
    },
    Reaction {
        added: bool,
        message_id: Snowflake,
        emoji: String,
        count: u32,
        user_id: Option<Snowflake>,
        user: Option<User>,
        server_id: Option<Snowflake>,
        channel_id: Option<Snowflake>,
        conversation_id: Option<Snowflake>,
    },
    Presence {
        user_id: Snowflake,
        status: String,
    },
    ChannelCreate(Value),
    ChannelUpdate(Value),
    ChannelDelete(Value),
    ConversationCreate(Value),
    ConversationUpdate(Value),
    ConversationDelete(Value),
    MemberAdd(Value),
    MemberRemove(Value),
    FriendRequest(Value),
    Raw {
        packet_type: String,
        payload: Value,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WsTicket {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadInit {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub part_size: usize,
    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadResult {
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub attachment: Option<Attachment>,
}
