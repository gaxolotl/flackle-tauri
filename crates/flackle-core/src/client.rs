use crate::error::{Error, Result};
use crate::gateway::{self, Command, Pending, Shared};
use crate::models::*;
use crate::rest::{RestClient, Tokens};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{sleep, timeout, Duration};

const ACK_TIMEOUT: Duration = Duration::from_secs(20);

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Guild {
        server_id: Snowflake,
        channel_id: Snowflake,
    },
    Conversation {
        conversation_id: Snowflake,
    },
}

impl Target {
    pub fn is_guild(&self) -> bool {
        matches!(self, Target::Guild { .. })
    }

    pub fn message_path(&self) -> String {
        match self {
            Target::Guild {
                server_id,
                channel_id,
            } => format!("/api/servers/{server_id}/channels/{channel_id}/messages"),
            Target::Conversation { conversation_id } => {
                format!("/api/conversations/{conversation_id}/messages")
            }
        }
    }

    fn attachments_path(&self) -> String {
        match self {
            Target::Guild {
                server_id,
                channel_id,
            } => format!("/api/servers/{server_id}/channels/{channel_id}/attachments"),
            Target::Conversation { conversation_id } => {
                format!("/api/conversations/{conversation_id}/attachments")
            }
        }
    }

    fn reaction_path(&self, message_id: &str, emoji: &str, mine: bool) -> String {
        let emoji = encode(emoji);
        let base = format!("{}/{message_id}/reactions/{emoji}", self.message_path());
        if mine && self.is_guild() {
            format!("{base}/@me")
        } else {
            base
        }
    }
}

fn encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn nonce() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{stamp:x}{counter:x}")
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginOutcome {
    Success { user: SelfUser },
    MfaRequired { mfa_token: String, methods: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Bootstrap {
    pub me: SelfUser,
    pub servers: Vec<Server>,
    pub conversations: Vec<Conversation>,
    pub unread: UnreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FetchMessagesOptions {
    pub limit: Option<u32>,
    pub before: Option<Snowflake>,
    pub after: Option<Snowflake>,
    pub around: Option<Snowflake>,
}

#[derive(Debug, Deserialize, Default)]
struct LoginResponse {
    #[serde(default)]
    user: Option<SelfUser>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    mfa_required: bool,
    #[serde(default)]
    mfa_token: Option<String>,
    #[serde(default)]
    methods: Vec<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ServersResponse {
    #[serde(default)]
    servers: Vec<Server>,
}

pub(crate) struct Inner {
    pub rest: RestClient,
    pub shared: Arc<Shared>,
    pub gateway_tx: Mutex<Option<mpsc::UnboundedSender<Command>>>,
    pub auto_subscribe: bool,
}

#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    pub fn new() -> Self {
        Self::with_rest(RestClient::new())
    }

    pub fn with_rest(rest: RestClient) -> Self {
        let shared = Arc::new(Shared::new(None));
        Self {
            inner: Arc::new(Inner {
                rest,
                shared,
                gateway_tx: Mutex::new(None),
                auto_subscribe: true,
            }),
        }
    }

    pub fn rest(&self) -> &RestClient {
        &self.inner.rest
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.inner.shared.events.subscribe()
    }

    pub fn is_connected(&self) -> bool {
        self.inner.shared.connected.load(Ordering::Relaxed)
    }

    pub fn is_authenticated(&self) -> bool {
        self.inner.rest.has_session()
    }

    pub fn tokens(&self) -> Tokens {
        self.inner.rest.tokens()
    }

    pub fn set_token_saver(&self, saver: Arc<dyn Fn(&Tokens) + Send + Sync>) {
        self.inner.rest.set_token_saver(saver);
    }

    /// Restores a previously saved session and returns the account.
    pub async fn restore(&self, tokens: Tokens) -> Result<SelfUser> {
        self.inner.rest.set_tokens(&tokens);
        let user = self.fetch_me().await?;
        self.inner.rest.save_tokens();
        Ok(user)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginOutcome> {
        self.inner.rest.ensure_csrf().await;
        let response: LoginResponse = self
            .inner
            .rest
            .post_anonymous(
                "/api/auth/login",
                json!({ "email": email, "password": password }),
            )
            .await?;
        if response.mfa_required {
            return Ok(LoginOutcome::MfaRequired {
                mfa_token: response
                    .mfa_token
                    .ok_or_else(|| Error::Other("MFA challenge did not include a token".into()))?,
                methods: response.methods,
            });
        }
        Ok(LoginOutcome::Success {
            user: self.complete_login(response).await?,
        })
    }

    pub async fn login_mfa(
        &self,
        mfa_token: &str,
        code: &str,
        backup_code: bool,
    ) -> Result<SelfUser> {
        let body = if backup_code {
            json!({ "mfa_token": mfa_token, "backup_code": code })
        } else {
            json!({ "mfa_token": mfa_token, "code": code })
        };
        let response: LoginResponse = self
            .inner
            .rest
            .post_anonymous("/api/auth/login/mfa", body)
            .await?;
        self.complete_login(response).await
    }

    async fn complete_login(&self, response: LoginResponse) -> Result<SelfUser> {
        let access_token = response
            .access_token
            .or_else(|| self.inner.rest.tokens().access_token)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| {
                Error::Other(
                    response
                        .error
                        .clone()
                        .unwrap_or_else(|| "login returned no access token".to_string()),
                )
            })?;
        let mut tokens = self.inner.rest.tokens();
        tokens.access_token = Some(access_token);
        self.inner.rest.set_tokens(&tokens);

        let user = match response.user {
            Some(user) => user,
            None => self.fetch_me().await?,
        };
        self.set_user(user.clone());
        Ok(user)
    }

    fn set_user(&self, user: SelfUser) {
        *self.inner.shared.user.write().unwrap() = Some(user);
    }

    pub async fn fetch_me(&self) -> Result<SelfUser> {
        let user: SelfUser = self.inner.rest.get("/api/auth/me", &[]).await?;
        self.set_user(user.clone());
        Ok(user)
    }

    pub async fn fetch_servers(&self) -> Result<Vec<Server>> {
        let data: ServersResponse = self.inner.rest.get("/api/servers", &[]).await?;
        Ok(data.servers)
    }

    pub async fn fetch_server(&self, server_id: &str) -> Result<Server> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}"), &[])
            .await
    }

    pub async fn fetch_channels(&self, server_id: &str) -> Result<ChannelBundle> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/channels"), &[])
            .await
    }

    pub async fn fetch_channel_kinds(&self) -> Result<Vec<String>> {
        let data: Value = self
            .inner
            .rest
            .get("/api/channel-kinds/creatable", &[])
            .await?;
        Ok(data
            .get("types")
            .and_then(Value::as_array)
            .map(|types| {
                types
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn fetch_channel_analytics_bundle(
        &self,
        server_id: &str,
        channel_id: &str,
    ) -> Result<Value> {
        self.inner
            .rest
            .get(
                &format!("/api/servers/{server_id}/channels/{channel_id}/analytics/bundle"),
                &[],
            )
            .await
    }

    pub async fn fetch_channel_activity_calendar(
        &self,
        server_id: &str,
        channel_id: &str,
        year: u32,
    ) -> Result<Value> {
        self.inner
            .rest
            .get(
                &format!("/api/servers/{server_id}/channels/{channel_id}/analytics/calendar"),
                &[("year", year.to_string())],
            )
            .await
    }

    pub async fn fetch_channel_message_punchcard(
        &self,
        server_id: &str,
        channel_id: &str,
        days: u32,
    ) -> Result<Value> {
        self.inner
            .rest
            .get(
                &format!("/api/servers/{server_id}/channels/{channel_id}/analytics/punchcard"),
                &[("days", days.to_string())],
            )
            .await
    }

    pub async fn fetch_conversations(&self) -> Result<Vec<Conversation>> {
        self.inner.rest.get("/api/conversations", &[]).await
    }

    pub async fn fetch_friends(&self) -> Result<Vec<User>> {
        self.inner.rest.get("/api/friends", &[]).await
    }

    pub async fn fetch_friend_requests(&self) -> Result<Value> {
        self.inner
            .rest
            .get("/api/friends/requests", &[])
            .await
    }

    pub async fn add_friend(&self, username: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post("/api/friends/requests", json!({ "username": username }))
            .await?;
        Ok(())
    }

    pub async fn accept_friend_request(&self, request_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post(&format!("/api/friends/requests/{request_id}/accept"), json!({}))
            .await?;
        Ok(())
    }

    pub async fn reject_friend_request(&self, request_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post(&format!("/api/friends/requests/{request_id}/reject"), json!({}))
            .await?;
        Ok(())
    }

    pub async fn cancel_friend_request(&self, request_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/friends/requests/{request_id}"))
            .await?;
        Ok(())
    }

    pub async fn remove_friend(&self, user_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/friends/{user_id}"))
            .await?;
        Ok(())
    }

    pub async fn create_dm(&self, user_id: &str) -> Result<Conversation> {
        self.inner
            .rest
            .post("/api/conversations", json!({ "member_ids": [user_id] }))
            .await
    }

    pub async fn fetch_blocked(&self) -> Result<Vec<User>> {
        let data: Value = self.inner.rest.get("/api/friends/blocked", &[]).await?;
        if data.is_array() {
            return Ok(serde_json::from_value(data).unwrap_or_default());
        }
        for key in ["users", "blocked", "blocked_users"] {
            if let Some(value) = data.get(key) {
                if let Ok(users) = serde_json::from_value(value.clone()) {
                    return Ok(users);
                }
            }
        }
        Ok(Vec::new())
    }

    pub async fn update_server(&self, server_id: &str, patch: ServerUpdate) -> Result<Server> {
        let body = serde_json::to_value(&patch)
            .map_err(|error| Error::Other(error.to_string()))?;
        self.inner
            .rest
            .patch(&format!("/api/servers/{server_id}"), body)
            .await
    }

    pub async fn upload_server_icon(
        &self,
        server_id: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<String> {
        let value = self
            .inner
            .rest
            .upload_file(
                &format!("/api/servers/{server_id}/icon"),
                "image",
                file_name,
                content_type,
                bytes,
            )
            .await?;
        value
            .get("url")
            .or_else(|| value.get("icon_url"))
            .or_else(|| value.get("banner_url"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Other("icon upload returned no URL".to_string()))
    }

    pub async fn create_channel(
        &self,
        server_id: &str,
        options: CreateChannel,
    ) -> Result<Channel> {
        let body = serde_json::to_value(&options)
            .map_err(|error| Error::Other(error.to_string()))?;
        self.inner
            .rest
            .post(&format!("/api/servers/{server_id}/channels"), body)
            .await
    }

    pub async fn create_category(&self, server_id: &str, name: &str) -> Result<Category> {
        self.inner
            .rest
            .post(
                &format!("/api/servers/{server_id}/categories"),
                json!({ "name": name }),
            )
            .await
    }

    pub async fn update_channel(
        &self,
        server_id: &str,
        channel_id: &str,
        patch: ChannelUpdate,
    ) -> Result<Channel> {
        let body = serde_json::to_value(&patch)
            .map_err(|error| Error::Other(error.to_string()))?;
        self.inner
            .rest
            .patch(
                &format!("/api/servers/{server_id}/channels/{channel_id}"),
                body,
            )
            .await
    }

    pub async fn update_channel_positions(&self, server_id: &str, positions: Value) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .put(
                &format!("/api/servers/{server_id}/channels/positions"),
                json!({ "channels": positions }),
            )
            .await?;
        Ok(())
    }

    pub async fn delete_channel(&self, server_id: &str, channel_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/servers/{server_id}/channels/{channel_id}"))
            .await?;
        Ok(())
    }

    pub async fn fetch_roles(&self, server_id: &str) -> Result<RolesResponse> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/roles"), &[])
            .await
    }

    pub async fn update_roles(&self, server_id: &str, batch: Value) -> Result<Value> {
        self.inner
            .rest
            .patch(&format!("/api/servers/{server_id}/roles/batch"), batch)
            .await
    }

    pub async fn upload_role_icon(
        &self,
        server_id: &str,
        role_id: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<Value> {
        let changes = json!({
            "updates": [{ "id": role_id, "upload_icon": true }]
        });
        self.inner
            .rest
            .patch_role_icon(
                &format!("/api/servers/{server_id}/roles/batch"),
                role_id,
                file_name,
                content_type,
                bytes,
                changes,
            )
            .await
    }

    pub async fn fetch_server_invites(&self, server_id: &str) -> Result<Value> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/invites"), &[])
            .await
    }

    pub async fn delete_server_invite(&self, server_id: &str, code: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/servers/{server_id}/invites/{code}"))
            .await?;
        Ok(())
    }

    pub async fn kick_member(
        &self,
        server_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<()> {
        let _: Value = if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
            self.inner
                .rest
                .delete_with_body(
                    &format!("/api/servers/{server_id}/members/{user_id}"),
                    json!({ "reason": reason }),
                )
                .await?
        } else {
            self.inner
                .rest
                .delete(&format!("/api/servers/{server_id}/members/{user_id}"))
                .await?
        };
        Ok(())
    }

    pub async fn fetch_bans(&self, server_id: &str) -> Result<Value> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/bans"), &[])
            .await
    }

    pub async fn ban_member(
        &self,
        server_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<()> {
        let mut body = json!({ "user_id": user_id });
        if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
            body["reason"] = Value::String(reason);
        }
        let _: Value = self
            .inner
            .rest
            .post(&format!("/api/servers/{server_id}/bans"), body)
            .await?;
        Ok(())
    }

    pub async fn unban_member(&self, server_id: &str, user_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/servers/{server_id}/bans/{user_id}"))
            .await?;
        Ok(())
    }

    pub async fn fetch_audit_log(
        &self,
        server_id: &str,
        limit: u32,
        before: Option<String>,
    ) -> Result<Value> {
        let mut query = vec![("limit", limit.to_string())];
        if let Some(before) = before.filter(|value| !value.is_empty()) {
            query.push(("before", before));
        }
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/audit-log"), &query)
            .await
    }

    pub async fn fetch_lifts(&self, server_id: &str) -> Result<Value> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/lifts"), &[])
            .await
    }

    pub async fn fetch_lift_supporters(&self, server_id: &str) -> Result<Value> {
        self.inner
            .rest
            .get(&format!("/api/servers/{server_id}/lifts/supporters"), &[])
            .await
    }

    pub async fn fetch_saved_media(&self, server_id: &str) -> Result<Value> {
        self.inner
            .rest
            .get(
                "/api/media/saved",
                &[("server_id", server_id.to_string())],
            )
            .await
    }

    pub async fn update_category(
        &self,
        server_id: &str,
        category_id: &str,
        patch: Value,
    ) -> Result<Category> {
        self.inner
            .rest
            .patch(
                &format!("/api/servers/{server_id}/categories/{category_id}"),
                patch,
            )
            .await
    }

    pub async fn delete_category(&self, server_id: &str, category_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!(
                "/api/servers/{server_id}/categories/{category_id}"
            ))
            .await?;
        Ok(())
    }

    pub async fn fetch_server_permissions(&self, server_id: &str) -> Result<String> {
        let response: PermissionResponse = self
            .inner
            .rest
            .get(&format!("/api/servers/{server_id}/permissions/@me"), &[])
            .await?;
        Ok(response.permissions)
    }

    pub async fn fetch_server_settings(&self) -> Result<ServerSettingsBundle> {
        self.inner
            .rest
            .get("/api/servers/@me/settings", &[])
            .await
    }

    pub async fn fetch_channel_permissions(
        &self,
        server_id: &str,
        channel_id: &str,
    ) -> Result<String> {
        let response: PermissionResponse = self
            .inner
            .rest
            .get(
                &format!("/api/servers/{server_id}/channels/{channel_id}/permissions/@me"),
                &[],
            )
            .await?;
        Ok(response.permissions)
    }

    pub async fn fetch_unread_summary(&self) -> Result<UnreadSummary> {
        self.inner
            .rest
            .get("/api/servers/@me/unread-summary", &[])
            .await
    }

    pub async fn fetch_messages(
        &self,
        target: &Target,
        options: FetchMessagesOptions,
    ) -> Result<MessagePage> {
        let mut query: Vec<(&str, String)> = Vec::new();
        query.push(("limit", options.limit.unwrap_or(50).to_string()));
        if let Some(before) = options.before {
            query.push(("before", before));
        }
        if let Some(after) = options.after {
            query.push(("after", after));
        }
        if let Some(around) = options.around {
            query.push(("around", around));
        }
        self.inner
            .rest
            .get(&target.message_path(), &query)
            .await
    }

    pub async fn fetch_members_page(
        &self,
        server_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<MemberPage> {
        self.inner
            .rest
            .get(
                &format!("/api/servers/{server_id}/members"),
                &[
                    ("limit", limit.to_string()),
                    ("offset", offset.to_string()),
                ],
            )
            .await
    }

    pub async fn fetch_all_members(&self, server_id: &str) -> Result<Vec<Member>> {
        let mut members = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for page in 0..100u32 {
            let data = self.fetch_members_page(server_id, 100, page * 100).await?;
            if data.members.is_empty() {
                break;
            }
            for member in &data.members {
                if seen.insert(member.id().to_string()) {
                    members.push(member.clone());
                }
            }
            if data.members.len() < 100 {
                break;
            }
        }
        Ok(members)
    }

    pub async fn set_member_roles(
        &self,
        server_id: &str,
        user_id: &str,
        add: Vec<String>,
        remove: Vec<String>,
    ) -> Result<Member> {
        self.inner
            .rest
            .patch(
                &format!("/api/servers/{server_id}/members/{user_id}/roles"),
                json!({ "add": add, "remove": remove }),
            )
            .await
    }

    pub async fn fetch_member_list_snapshot(
        &self,
        server_id: &str,
        channel_id: &str,
    ) -> Result<MemberListSnapshot> {
        self.inner
            .rest
            .get(
                &format!(
                    "/api/servers/{server_id}/channels/{channel_id}/member-list-snapshot"
                ),
                &[("locale", "en-GB".to_string())],
            )
            .await
    }

    pub async fn accept_server_invite(&self, code: &str) -> Result<AcceptInviteResult> {
        self.inner
            .rest
            .post(
                &format!("/api/server-invites/{code}/accept"),
                json!({}),
            )
            .await
    }

    pub async fn create_server(&self, name: &str) -> Result<CreateServerResult> {
        self.inner
            .rest
            .post("/api/servers", json!({ "name": name }))
            .await
    }

    pub async fn mark_server_read(&self, server_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post(&format!("/api/servers/{server_id}/mark-read"), json!({}))
            .await?;
        Ok(())
    }

    pub async fn update_server_user_settings(
        &self,
        server_id: &str,
        patch: Value,
    ) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .patch(&format!("/api/servers/{server_id}/settings"), patch)
            .await?;
        Ok(())
    }

    pub async fn update_server_positions(&self, servers: Value) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .put("/api/servers/@me/settings", json!({ "servers": servers }))
            .await?;
        Ok(())
    }

    pub async fn leave_server(&self, server_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/servers/{server_id}/members/@me"))
            .await?;
        Ok(())
    }

    pub async fn create_server_invite(
        &self,
        server_id: &str,
        options: Value,
    ) -> Result<Invite> {
        self.inner
            .rest
            .post(&format!("/api/servers/{server_id}/invites"), options)
            .await
    }

    pub async fn block_user(&self, user_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post("/api/friends/block", json!({ "user_id": user_id }))
            .await?;
        Ok(())
    }

    pub async fn unblock_user(&self, user_id: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .delete(&format!("/api/friends/block/{user_id}"))
            .await?;
        Ok(())
    }

    pub async fn report_user(
        &self,
        user_id: &str,
        category: &str,
        additional_info: &str,
    ) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .post(
                "/api/reports",
                json!({
                    "report_type": "user",
                    "reported_user_id": user_id,
                    "category": category,
                    "additional_info": additional_info,
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn fetch_sessions(&self) -> Result<Value> {
        self.inner.rest.get("/api/auth/sessions", &[]).await
    }

    pub async fn revoke_session(&self, session_id: &str, code: Option<String>) -> Result<()> {
        let body = match code.filter(|value| !value.trim().is_empty()) {
            Some(code) => json!({ "code": code }),
            None => json!({}),
        };
        let _: Value = self
            .inner
            .rest
            .delete_with_body(
                &format!("/api/auth/sessions/{session_id}"),
                body,
            )
            .await?;
        Ok(())
    }

    pub async fn revoke_all_sessions(&self, code: Option<String>) -> Result<()> {
        let body = match code.filter(|value| !value.trim().is_empty()) {
            Some(code) => json!({ "code": code }),
            None => json!({}),
        };
        let _: Value = self
            .inner
            .rest
            .delete_with_body("/api/auth/sessions", body)
            .await?;
        Ok(())
    }

    pub async fn fetch_security_events(&self) -> Result<Value> {
        self.inner
            .rest
            .get("/api/auth/security-events", &[])
            .await
    }

    pub async fn clear_security_events(&self, code: Option<String>) -> Result<()> {
        let body = match code.filter(|value| !value.trim().is_empty()) {
            Some(code) => json!({ "code": code }),
            None => json!({}),
        };
        let _: Value = self
            .inner
            .rest
            .post("/api/auth/security-events/clear", body)
            .await?;
        Ok(())
    }

    pub async fn fetch_2fa_status(&self) -> Result<Value> {
        self.inner.rest.get("/api/auth/2fa", &[]).await
    }

    pub async fn setup_2fa(&self, password: &str) -> Result<Value> {
        self.inner
            .rest
            .post("/api/auth/2fa/setup", json!({ "password": password }))
            .await
    }

    pub async fn confirm_2fa(&self, code: &str) -> Result<Value> {
        self.inner
            .rest
            .post("/api/auth/2fa/confirm", json!({ "code": code }))
            .await
    }

    pub async fn disable_2fa(
        &self,
        password: &str,
        code: Option<String>,
        backup_code: Option<String>,
    ) -> Result<Value> {
        let body = if let Some(backup_code) = backup_code.filter(|value| !value.trim().is_empty()) {
            json!({ "password": password, "backup_code": backup_code })
        } else {
            json!({
                "password": password,
                "code": code.unwrap_or_default(),
            })
        };
        self.inner
            .rest
            .post("/api/auth/2fa/disable", body)
            .await
    }

    pub async fn create_backup_codes(&self, password: &str, code: &str) -> Result<Value> {
        self.inner
            .rest
            .post(
                "/api/auth/2fa/backup-codes",
                json!({ "password": password, "code": code }),
            )
            .await
    }

    pub async fn connect(&self) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }
        if !self.inner.rest.has_session() {
            return Err(Error::NotAuthenticated);
        }

        if self.inner.auto_subscribe {
            let servers = self.fetch_servers().await.unwrap_or_default();
            let ids = servers.into_iter().map(|server| server.id).collect();
            *self.inner.shared.servers.lock().unwrap() = ids;
        }

        {
            let mut slot = self.inner.gateway_tx.lock().unwrap();
            let needs_spawn = slot
                .as_ref()
                .map(|sender| sender.is_closed())
                .unwrap_or(true);
            if needs_spawn {
                let (sender, receiver) = mpsc::unbounded_channel();
                *slot = Some(sender);
                let rest = self.inner.rest.clone();
                let shared = self.inner.shared.clone();
                tokio::spawn(gateway::run(rest, shared, receiver));
            }
        }

        for _ in 0..150 {
            if self.is_connected() {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        Err(Error::GatewayClosed)
    }

    pub async fn disconnect(&self) {
        if let Some(sender) = self.inner.gateway_tx.lock().unwrap().take() {
            let _ = sender.send(Command::Close);
        }
    }

    fn send_frame(&self, value: Value) -> Result<()> {
        let sender = self.inner.gateway_tx.lock().unwrap().clone();
        let Some(sender) = sender else {
            return Err(Error::GatewayClosed);
        };
        sender
            .send(Command::Frame(value))
            .map_err(|_| Error::GatewayClosed)
    }

    pub async fn send_message(
        &self,
        target: &Target,
        content: String,
        attachments: Vec<Attachment>,
        reply_to: Option<String>,
    ) -> Result<Message> {
        if content.is_empty() && attachments.is_empty() {
            return Err(Error::Other(
                "message content or at least one attachment is required".into(),
            ));
        }
        if !self.is_connected() {
            return Err(Error::GatewayClosed);
        }

        let nonce = nonce();
        let (tx, rx) = oneshot::channel();
        self.inner.shared.pending.lock().unwrap().insert(
            nonce.clone(),
            Pending::Send {
                tx,
                content: content.clone(),
                message_id: None,
                attachments: if attachments.is_empty() {
                    None
                } else {
                    Some(attachments.clone())
                },
                embeds: None,
                acked: false,
            },
        );

        let mut payload = json!({ "content": content, "nonce": nonce });
        match target {
            Target::Guild {
                server_id,
                channel_id,
            } => {
                payload["server_id"] = json!(server_id);
                payload["channel_id"] = json!(channel_id);
            }
            Target::Conversation { conversation_id } => {
                payload["conversation_id"] = json!(conversation_id);
            }
        }
        if !attachments.is_empty() {
            payload["attachments"] = serde_json::to_value(&attachments).unwrap_or(Value::Null);
        }
        if let Some(reply) = reply_to {
            payload["reply_to_message_id"] = json!(reply);
        }

        let frame = if target.is_guild() {
            json!({ "type": "send_channel_message", "payload": payload })
        } else {
            json!({ "type": "send_message", "payload": payload })
        };
        if let Err(error) = self.send_frame(frame) {
            self.inner.shared.pending.lock().unwrap().remove(&nonce);
            return Err(error);
        }

        match timeout(ACK_TIMEOUT, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::Other("gateway closed while sending".into())),
            Err(_) => {
                self.inner.shared.pending.lock().unwrap().remove(&nonce);
                Err(Error::AckTimeout)
            }
        }
    }

    pub async fn edit_message(
        &self,
        target: &Target,
        message_id: &str,
        content: String,
    ) -> Result<()> {
        self.message_op(target, message_id, content, "edit").await
    }

    pub async fn delete_message(&self, target: &Target, message_id: &str) -> Result<()> {
        self.message_op(target, message_id, String::new(), "delete")
            .await
    }

    async fn message_op(
        &self,
        target: &Target,
        message_id: &str,
        content: String,
        action: &str,
    ) -> Result<()> {
        if !self.is_connected() {
            return Err(Error::GatewayClosed);
        }
        let nonce = nonce();
        let (tx, rx) = oneshot::channel();
        self.inner
            .shared
            .pending
            .lock()
            .unwrap()
            .insert(nonce.clone(), Pending::Op { tx });

        let mut payload = json!({ "message_id": message_id, "nonce": nonce });
        match target {
            Target::Guild {
                server_id,
                channel_id,
            } => {
                payload["server_id"] = json!(server_id);
                payload["channel_id"] = json!(channel_id);
            }
            Target::Conversation { conversation_id } => {
                payload["conversation_id"] = json!(conversation_id);
            }
        }
        let (kind, payload) = match (action, target.is_guild()) {
            ("edit", true) => {
                payload["content"] = json!(content);
                ("edit_channel_message", payload)
            }
            ("edit", false) => {
                payload["content"] = json!(content);
                ("edit_message", payload)
            }
            (_, true) => ("delete_channel_message", payload),
            (_, false) => ("delete_message", payload),
        };

        if let Err(error) = self.send_frame(json!({ "type": kind, "payload": payload })) {
            self.inner.shared.pending.lock().unwrap().remove(&nonce);
            return Err(error);
        }
        match timeout(ACK_TIMEOUT, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::Other("gateway closed".into())),
            Err(_) => {
                self.inner.shared.pending.lock().unwrap().remove(&nonce);
                Err(Error::AckTimeout)
            }
        }
    }

    pub async fn send_typing(&self, target: &Target) -> Result<()> {
        let (kind, payload) = match target {
            Target::Guild {
                server_id,
                channel_id,
            } => (
                "channel_typing_start",
                json!({ "server_id": server_id, "channel_id": channel_id }),
            ),
            Target::Conversation { conversation_id } => (
                "typing_start",
                json!({ "conversation_id": conversation_id }),
            ),
        };
        self.send_frame(json!({ "type": kind, "payload": payload }))
    }

    pub async fn stop_typing(&self, target: &Target) -> Result<()> {
        let (kind, payload) = match target {
            Target::Guild {
                server_id,
                channel_id,
            } => (
                "channel_typing_stop",
                json!({ "server_id": server_id, "channel_id": channel_id }),
            ),
            Target::Conversation { conversation_id } => (
                "typing_stop",
                json!({ "conversation_id": conversation_id }),
            ),
        };
        self.send_frame(json!({ "type": kind, "payload": payload }))
    }

    pub async fn mark_read(&self, target: &Target, message_id: &str) -> Result<()> {
        let (kind, payload) = match target {
            Target::Guild {
                server_id,
                channel_id,
            } => (
                "mark_channel_read_channel",
                json!({
                    "server_id": server_id,
                    "channel_id": channel_id,
                    "message_id": message_id
                }),
            ),
            Target::Conversation { conversation_id } => (
                "mark_read",
                json!({ "conversation_id": conversation_id, "message_id": message_id }),
            ),
        };
        self.send_frame(json!({ "type": kind, "payload": payload }))
    }

    pub async fn react(&self, target: &Target, message_id: &str, emoji: &str) -> Result<()> {
        let path = target.reaction_path(message_id, emoji, true);
        let _: Value = self.inner.rest.put_empty(&path).await?;
        Ok(())
    }

    pub async fn unreact(&self, target: &Target, message_id: &str, emoji: &str) -> Result<()> {
        let path = target.reaction_path(message_id, emoji, true);
        let _: Value = self.inner.rest.delete(&path).await?;
        Ok(())
    }

    pub async fn upload_attachment(
        &self,
        target: &Target,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<Vec<Attachment>> {
        let value = self
            .inner
            .rest
            .upload_attachment(
                &target.attachments_path(),
                file_name,
                content_type,
                bytes,
            )
            .await?;
        let result: UploadResult = serde_json::from_value(value)?;
        Ok(if result.attachments.is_empty() {
            result.attachment.into_iter().collect()
        } else {
            result.attachments
        })
    }

    pub async fn set_status(&self, status: &str) -> Result<()> {
        let _: Value = self
            .inner
            .rest
            .patch("/api/status", json!({ "status": status }))
            .await?;
        Ok(())
    }

    pub async fn fetch_status(&self) -> Result<String> {
        let value: Value = self.inner.rest.get("/api/status", &[]).await?;
        Ok(value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("online")
            .to_string())
    }

    pub async fn update_typing_indicator(
        &self,
        enabled: bool,
        text: Option<&str>,
        style: Option<&str>,
        emojis: Option<Vec<String>>,
    ) -> Result<()> {
        let mut body = json!({ "enabled": enabled });
        if enabled {
            if let Some(text) = text {
                body["text"] = json!(text);
            }
            if let Some(style) = style {
                body["style"] = json!(style);
            }
            if style == Some("emojis") {
                if let Some(emojis) = emojis {
                    body["emojis"] = json!(emojis);
                }
            }
        }
        let _: Value = self
            .inner
            .rest
            .patch("/api/users/typing-indicator", body)
            .await?;
        Ok(())
    }

    pub async fn create_link_preview(&self, url: &str) -> Result<Value> {
        self.inner
            .rest
            .get("/api/link-preview", &[("url", url.to_string())])
            .await
    }

    pub async fn update_message_embeds(
        &self,
        target: &Target,
        message_id: &str,
        tokens: Vec<String>,
    ) -> Result<Value> {
        let path = format!("{}/{message_id}/embeds", target.message_path());
        self.inner.rest.put(&path, json!({ "tokens": tokens })).await
    }

    pub async fn set_idle(&self, idle: bool) -> Result<()> {
        self.send_frame(json!({
            "type": "set_idle",
            "payload": { "idle": idle }
        }))
    }

    pub async fn bootstrap(&self) -> Result<Bootstrap> {
        let me = self.fetch_me().await?;
        let servers = self.fetch_servers().await.unwrap_or_default();
        let conversations = self.fetch_conversations().await.unwrap_or_default();
        let unread = self.fetch_unread_summary().await.unwrap_or_default();
        Ok(Bootstrap {
            me,
            servers,
            conversations,
            unread,
        })
    }

    pub async fn logout(&self) -> Result<()> {
        let result: Result<Value> = self.inner.rest.post("/api/auth/logout", json!({})).await;
        self.disconnect().await;
        self.inner.rest.clear_session();
        *self.inner.shared.user.write().unwrap() = None;
        result.map(|_| ())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
