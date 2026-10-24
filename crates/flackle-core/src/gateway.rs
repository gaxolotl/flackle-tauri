use crate::error::{Error, Result};
use crate::models::*;
use crate::rest::RestClient;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{interval_at, sleep, MissedTickBehavior};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const ACK_GRACE: Duration = Duration::from_millis(1500);
const HEARTBEAT: Duration = Duration::from_secs(45);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(65);

pub(crate) enum Pending {
    Send {
        tx: oneshot::Sender<Result<Message>>,
        content: String,
        message_id: Option<String>,
        attachments: Option<Vec<Attachment>>,
        embeds: Option<Vec<Value>>,
        acked: bool,
    },
    Op {
        tx: oneshot::Sender<Result<()>>,
    },
}

pub(crate) struct Shared {
    pub events: broadcast::Sender<GatewayEvent>,
    pub pending: Mutex<HashMap<String, Pending>>,
    pub user: RwLock<Option<SelfUser>>,
    pub connected: AtomicBool,
    pub servers: Mutex<Vec<String>>,
    pub session_id: RwLock<Option<String>>,
    start: Instant,
    last_activity_ms: AtomicU64,
}

impl Shared {
    pub fn new(user: Option<SelfUser>) -> Self {
        let (events, _) = broadcast::channel(512);
        Self {
            events,
            pending: Mutex::new(HashMap::new()),
            user: RwLock::new(user),
            connected: AtomicBool::new(false),
            servers: Mutex::new(Vec::new()),
            session_id: RwLock::new(None),
            start: Instant::now(),
            last_activity_ms: AtomicU64::new(0),
        }
    }

    pub fn touch(&self) {
        self.last_activity_ms
            .store(self.start.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    fn idle_for(&self) -> Duration {
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        let now = self.start.elapsed().as_millis() as u64;
        Duration::from_millis(now.saturating_sub(last))
    }

    pub fn emit(&self, event: GatewayEvent) {
        let _ = self.events.send(event);
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Frame(Value),
    Close,
}

enum End {
    Manual,
    Closed {
        code: u16,
        reason: String,
        had_hello: bool,
    },
    Failed(String),
}

pub(crate) async fn run(
    rest: RestClient,
    shared: Arc<Shared>,
    mut rx: mpsc::UnboundedReceiver<Command>,
) {
    let mut attempts = 0u32;
    let mut fast_reconnects = 0u32;

    loop {
        let end = connect_once(&rest, &shared, &mut rx).await;
        shared.connected.store(false, Ordering::Relaxed);

        match end {
            End::Manual => break,
            End::Closed {
                code,
                reason,
                had_hello,
            } => {
                shared.emit(GatewayEvent::Disconnected {
                    code,
                    reason: reason.clone(),
                });
                if had_hello {
                    attempts = 0;
                    fast_reconnects = 0;
                }
                if code == 1000 || code == 4001 {
                    break;
                }
                if code == 1012 {
                    fast_reconnects = 8;
                }
            }
            End::Failed(message) => {
                shared.emit(GatewayEvent::Error { message });
            }
        }

        if attempts >= 10 {
            shared.emit(GatewayEvent::Error {
                message: "maximum reconnect attempts reached".to_string(),
            });
            break;
        }

        let delay = if fast_reconnects > 0 {
            fast_reconnects -= 1;
            Duration::from_millis(250 + (fast_reconnects as u64 * 50))
        } else {
            attempts += 1;
            Duration::from_millis(1000 * 2u64.pow(attempts.saturating_sub(1).min(4)))
        };

        tokio::select! {
            _ = sleep(delay) => {}
            command = rx.recv() => {
                match command {
                    Some(Command::Close) | None => break,
                    Some(Command::Frame(_)) => {}
                }
            }
        }
    }

    shared.connected.store(false, Ordering::Relaxed);
}

async fn connect_once(
    rest: &RestClient,
    shared: &Arc<Shared>,
    rx: &mut mpsc::UnboundedReceiver<Command>,
) -> End {
    let ticket = match rest.get::<WsTicket>("/api/ws/url", &[]).await {
        Ok(ticket) => ticket,
        Err(error) => return End::Failed(format!("could not get gateway ticket: {error}")),
    };
    if ticket.url.is_empty() || ticket.ticket.is_empty() {
        return End::Failed("gateway did not return a URL and ticket".to_string());
    }

    let mut url = match url::Url::parse(&ticket.url) {
        Ok(url) => url,
        Err(error) => return End::Failed(format!("invalid gateway URL: {error}")),
    };
    url.query_pairs_mut().append_pair("ticket", &ticket.ticket);

    let mut request = match url.as_str().into_client_request() {
        Ok(request) => request,
        Err(error) => return End::Failed(format!("invalid gateway request: {error}")),
    };
    if let Ok(origin) = HeaderValue::from_str(rest.base_url()) {
        request.headers_mut().insert("Origin", origin);
    }
    if let Ok(agent) = HeaderValue::from_str(rest.user_agent()) {
        request.headers_mut().insert("User-Agent", agent);
    }

    let (socket, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(value) => value,
        Err(error) => return End::Failed(format!("gateway connect failed: {error}")),
    };
    let (mut write, mut read) = socket.split();

    shared.touch();
    shared.emit(GatewayEvent::Connected);

    let servers = shared.servers.lock().unwrap().clone();
    let _ = send_json(&mut write, json!({ "type": "set_idle", "payload": { "idle": false } })).await;
    if !servers.is_empty() {
        let _ = send_json(
            &mut write,
            json!({ "type": "subscribe_servers", "payload": { "server_ids": servers } }),
        )
        .await;
    }

    let mut heartbeat = interval_at(tokio::time::Instant::now() + HEARTBEAT, HEARTBEAT);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut had_hello = false;

    loop {
        tokio::select! {
            command = rx.recv() => {
                match command {
                    Some(Command::Frame(value)) => {
                        if send_json(&mut write, value).await.is_err() {
                            return End::Failed("gateway write failed".to_string());
                        }
                    }
                    Some(Command::Close) | None => {
                        let _ = write.send(WsMessage::Close(None)).await;
                        return End::Manual;
                    }
                }
            }
            message = read.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_text(shared, &text, &mut had_hello);
                    }
                    Some(Ok(WsMessage::Binary(bytes))) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            handle_text(shared, &text, &mut had_hello);
                        }
                    }
                    Some(Ok(WsMessage::Ping(payload))) => {
                        let _ = write.send(WsMessage::Pong(payload)).await;
                    }
                    Some(Ok(WsMessage::Pong(_))) => shared.touch(),
                    Some(Ok(WsMessage::Close(frame))) => {
                        let (code, reason) = frame
                            .map(|frame| (u16::from(frame.code), frame.reason.to_string()))
                            .unwrap_or((1000, String::new()));
                        return End::Closed { code, reason, had_hello };
                    }
                    Some(Ok(WsMessage::Frame(_))) => {}
                    Some(Err(error)) => {
                        return End::Closed {
                            code: 1012,
                            reason: format!("socket error: {error}"),
                            had_hello,
                        };
                    }
                    None => {
                        return End::Closed {
                            code: 1006,
                            reason: "socket closed".to_string(),
                            had_hello,
                        };
                    }
                }
            }
            _ = heartbeat.tick() => {
                if shared.idle_for() > HEARTBEAT_TIMEOUT {
                    let _ = write.send(WsMessage::Close(None)).await;
                    return End::Closed { code: 4002, reason: "heartbeat stalled".to_string(), had_hello };
                }
                if send_json(&mut write, json!({ "type": "ping", "payload": { "idle": false } })).await.is_err() {
                    return End::Closed { code: 1012, reason: "heartbeat write failed".to_string(), had_hello };
                }
            }
        }
    }
}

async fn send_json<S>(write: &mut S, value: Value) -> std::result::Result<(), ()>
where
    S: SinkExt<WsMessage> + Unpin,
{
    write
        .send(WsMessage::Text(value.to_string()))
        .await
        .map_err(|_| ())
}

fn handle_text(shared: &Arc<Shared>, text: &str, had_hello: &mut bool) {
    shared.touch();
    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(packet) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(kind) = packet.get("type").and_then(Value::as_str) else {
            continue;
        };
        let payload = packet.get("payload").cloned().unwrap_or(Value::Null);
        dispatch(shared, kind, payload, had_hello);
    }
}

fn dispatch(shared: &Arc<Shared>, kind: &str, payload: Value, had_hello: &mut bool) {
    match kind {
        "hello" => {
            *had_hello = true;
            let session_id = payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let presences = payload
                .get("presences")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
                .unwrap_or_default();
            *shared.session_id.write().unwrap() = Some(session_id.clone());
            shared.connected.store(true, Ordering::Relaxed);
            shared.emit(GatewayEvent::Ready {
                session_id,
                user: shared.user.read().unwrap().clone(),
                presences,
            });
        }
        "pong" => {}
        "message_ack" => handle_ack(shared, &payload),
        "new_message" | "channel_message" => {
            let value = if kind == "new_message" {
                payload.clone()
            } else {
                payload.get("message").cloned().unwrap_or(Value::Null)
            };
            let Ok(message) = serde_json::from_value::<Message>(value) else {
                shared.emit(GatewayEvent::Raw {
                    packet_type: kind.to_string(),
                    payload,
                });
                return;
            };
            resolve_send(shared, &message);
            shared.emit(GatewayEvent::MessageCreate {
                message,
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                conversation_id: payload
                    .get("conversation_id")
                    .or_else(|| payload.get("message").and_then(|m| m.get("conversation_id")))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        "message_edited" | "channel_message_edited" => {
            shared.emit(GatewayEvent::MessageUpdate {
                id: payload
                    .get("message_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                content: payload
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                edited_at: payload
                    .get("edited_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                conversation_id: payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        "message_deleted" | "channel_message_deleted" => {
            shared.emit(GatewayEvent::MessageDelete {
                id: payload
                    .get("message_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                conversation_id: payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        "channel_messages_bulk_deleted" => {
            shared.emit(GatewayEvent::MessagesBulkDelete {
                ids: payload
                    .get("message_ids")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok())
                    .unwrap_or_default(),
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        "typing" | "channel_typing" => {
            shared.emit(GatewayEvent::Typing {
                user_id: payload
                    .get("user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                is_typing: payload
                    .get("is_typing")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                conversation_id: payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        "message_reaction_added"
        | "message_reaction_removed"
        | "channel_message_reaction_added"
        | "channel_message_reaction_removed" => {
            shared.emit(GatewayEvent::Reaction {
                added: kind.contains("added"),
                message_id: payload
                    .get("message_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                emoji: payload
                    .get("emoji")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                count: payload
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                user_id: payload
                    .get("user_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                user: payload
                    .get("user")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok()),
                server_id: payload
                    .get("server_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                channel_id: payload
                    .get("channel_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                conversation_id: payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        "presence_update" => {
            shared.emit(GatewayEvent::Presence {
                user_id: payload
                    .get("user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                status: payload
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("offline")
                    .to_string(),
            });
        }
        "channel_created" => shared.emit(GatewayEvent::ChannelCreate(payload)),
        "channel_updated" => shared.emit(GatewayEvent::ChannelUpdate(payload)),
        "channel_deleted" => shared.emit(GatewayEvent::ChannelDelete(payload)),
        "conversation_created" => shared.emit(GatewayEvent::ConversationCreate(payload)),
        "conversation_updated" => shared.emit(GatewayEvent::ConversationUpdate(payload)),
        "conversation_hidden" => shared.emit(GatewayEvent::ConversationDelete(payload)),
        "guild_member_added" | "member_added" => shared.emit(GatewayEvent::MemberAdd(payload)),
        "guild_member_removed" | "member_removed" | "member_left" => {
            shared.emit(GatewayEvent::MemberRemove(payload))
        }
        "friend_request_received" | "friend_request_accepted" => {
            shared.emit(GatewayEvent::FriendRequest(payload))
        }
        "account_restricted" => shared.emit(GatewayEvent::Error {
            message: "this account has been restricted".to_string(),
        }),
        _ => shared.emit(GatewayEvent::Raw {
            packet_type: kind.to_string(),
            payload,
        }),
    }
}

fn resolve_send(shared: &Shared, message: &Message) {
    let nonce = message.nonce.clone();
    let mut pending = shared.pending.lock().unwrap();
    if let Some(nonce) = nonce {
        if let Some(entry) = pending.remove(&nonce) {
            if let Pending::Send { tx, .. } = entry {
                let _ = tx.send(Ok(message.clone()));
            }
            return;
        }
    }
    if let Some((key, _)) = pending.iter().find(|(_, entry)| {
        matches!(entry, Pending::Send { message_id: Some(id), .. } if id == &message.id)
    }) {
        let key = key.clone();
        if let Some(Pending::Send { tx, .. }) = pending.remove(&key) {
            let _ = tx.send(Ok(message.clone()));
        }
    }
}

fn handle_ack(shared: &Arc<Shared>, payload: &Value) {
    let nonce = payload
        .get("nonce")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    match status.as_str() {
        "sent" => {
            let grace = {
                let mut pending = shared.pending.lock().unwrap();
                if let Some(Pending::Send {
                    message_id,
                    attachments,
                    embeds,
                    acked,
                    ..
                }) = pending.get_mut(&nonce)
                {
                    if *acked {
                        return;
                    }
                    *acked = true;
                    *message_id = payload
                        .get("message_id")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    if let Some(value) = payload.get("attachments") {
                        *attachments = serde_json::from_value(value.clone()).ok();
                    }
                    if let Some(value) = payload.get("embeds") {
                        *embeds = serde_json::from_value(value.clone()).ok();
                    }
                    true
                } else {
                    false
                }
            };
            if grace {
                let shared = shared.clone();
                tokio::spawn(async move {
                    sleep(ACK_GRACE).await;
                    let entry = shared.pending.lock().unwrap().remove(&nonce);
                    if let Some(Pending::Send {
                        tx,
                        content,
                        message_id,
                        attachments,
                        embeds,
                        ..
                    }) = entry
                    {
                        let now = chrono::Utc::now().to_rfc3339();
                        let message = Message {
                            id: message_id.unwrap_or_default(),
                            content,
                            kind: "text".to_string(),
                            created_at: now.clone(),
                            updated_at: now,
                            sender: shared.user.read().unwrap().as_ref().map(|u| u.user.clone()),
                            attachments: attachments.unwrap_or_default(),
                            embeds: embeds.unwrap_or_default(),
                            ..Message::default()
                        };
                        let _ = tx.send(Ok(message));
                    }
                });
            }
        }
        "edit_ok" | "delete_ok" => {
            let entry = shared.pending.lock().unwrap().remove(&nonce);
            if let Some(Pending::Op { tx }) = entry {
                let _ = tx.send(Ok(()));
            }
        }
        _ => {
            let entry = shared.pending.lock().unwrap().remove(&nonce);
            let message = payload
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("message failed: {status}"));
            match entry {
                Some(Pending::Send { tx, .. }) => {
                    let _ = tx.send(Err(Error::Other(message)));
                }
                Some(Pending::Op { tx }) => {
                    let _ = tx.send(Err(Error::Other(message)));
                }
                None => {}
            }
        }
    }
}
