use flackle_core::{
    AcceptInviteResult, Attachment, Bootstrap, Category, Channel, ChannelBundle, ChannelUpdate,
    Client, Conversation, CreateChannel, CreateServerResult, FetchMessagesOptions, Invite,
    LoginOutcome, Member, MemberListSnapshot, Message, MessagePage, RolesResponse, SelfUser,
    Server, ServerUpdate, Target, Tokens, UnreadSummary, User,
    ServerSettingsBundle,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

struct AppState {
    client: Client,
    data_dir: PathBuf,
}

const KEYRING_SERVICE: &str = "dev.flackle.desktop";
const KEYRING_USER: &str = "session";

fn error_text(error: flackle_core::Error) -> String {
    error.to_string()
}

fn save_tokens(data_dir: &PathBuf, tokens: &Tokens) -> Result<(), String> {
    let json = serde_json::to_string(tokens).map_err(|error| error.to_string())?;
    let path = data_dir.join("session.json");
    let mut stored = false;

    if std::fs::create_dir_all(data_dir).is_ok() {
        if std::fs::write(&path, &json).is_ok() {
            stored = true;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
        }
    }

    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if entry.set_password(&json).is_ok() {
            stored = true;
        }
    }

    if stored {
        Ok(())
    } else {
        Err(format!("could not save the session to {}", path.display()))
    }
}

fn load_tokens(data_dir: &PathBuf) -> Option<Tokens> {
    if let Ok(text) = std::fs::read_to_string(data_dir.join("session.json")) {
        if let Ok(tokens) = serde_json::from_str::<Tokens>(&text) {
            if tokens.refresh_token.is_some() || tokens.access_token.is_some() {
                return Some(tokens);
            }
        }
    }
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(json) = entry.get_password() {
            if let Ok(tokens) = serde_json::from_str::<Tokens>(&json) {
                return Some(tokens);
            }
        }
    }
    None
}

fn clear_tokens(data_dir: &PathBuf) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    let _ = std::fs::remove_file(data_dir.join("session.json"));
}

#[tauri::command]
async fn login(
    state: tauri::State<'_, AppState>,
    email: String,
    password: String,
) -> Result<LoginOutcome, String> {
    let outcome = state
        .client
        .login(&email, &password)
        .await
        .map_err(error_text)?;
    if matches!(outcome, LoginOutcome::Success { .. }) {
        let _ = save_tokens(&state.data_dir, &state.client.tokens());
    }
    Ok(outcome)
}

#[tauri::command]
async fn login_mfa(
    state: tauri::State<'_, AppState>,
    mfa_token: String,
    code: String,
    backup_code: Option<bool>,
) -> Result<SelfUser, String> {
    let user = state
        .client
        .login_mfa(&mfa_token, &code, backup_code.unwrap_or(false))
        .await
        .map_err(error_text)?;
    let _ = save_tokens(&state.data_dir, &state.client.tokens());
    Ok(user)
}

#[tauri::command]
async fn restore(state: tauri::State<'_, AppState>) -> Result<Option<SelfUser>, String> {
    let Some(tokens) = load_tokens(&state.data_dir) else {
        return Ok(None);
    };
    match state.client.restore(tokens).await {
        Ok(user) => {
            let _ = state.client.connect().await;
            Ok(Some(user))
        }
        Err(flackle_core::Error::SessionExpired) => {
            clear_tokens(&state.data_dir);
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
async fn logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let _ = state.client.logout().await;
    clear_tokens(&state.data_dir);
    Ok(())
}

#[tauri::command]
async fn bootstrap(state: tauri::State<'_, AppState>) -> Result<Bootstrap, String> {
    let bootstrap = state.client.bootstrap().await.map_err(error_text)?;
    let _ = state.client.connect().await;
    Ok(bootstrap)
}

#[tauri::command]
async fn fetch_servers(state: tauri::State<'_, AppState>) -> Result<Vec<Server>, String> {
    state.client.fetch_servers().await.map_err(error_text)
}

#[tauri::command]
async fn fetch_server(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Server, String> {
    state
        .client
        .fetch_server(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_channels(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<ChannelBundle, String> {
    state
        .client
        .fetch_channels(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Conversation>, String> {
    state
        .client
        .fetch_conversations()
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_friends(state: tauri::State<'_, AppState>) -> Result<Vec<User>, String> {
    state.client.fetch_friends().await.map_err(error_text)
}

#[tauri::command]
async fn fetch_blocked(state: tauri::State<'_, AppState>) -> Result<Vec<User>, String> {
    state.client.fetch_blocked().await.map_err(error_text)
}

#[tauri::command]
async fn fetch_friend_requests(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    state
        .client
        .fetch_friend_requests()
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn add_friend(
    state: tauri::State<'_, AppState>,
    username: String,
) -> Result<(), String> {
    state
        .client
        .add_friend(&username)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn accept_friend_request(
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> Result<(), String> {
    state
        .client
        .accept_friend_request(&request_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn reject_friend_request(
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> Result<(), String> {
    state
        .client
        .reject_friend_request(&request_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn cancel_friend_request(
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> Result<(), String> {
    state
        .client
        .cancel_friend_request(&request_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn remove_friend(
    state: tauri::State<'_, AppState>,
    user_id: String,
) -> Result<(), String> {
    state
        .client
        .remove_friend(&user_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_dm(
    state: tauri::State<'_, AppState>,
    user_id: String,
) -> Result<Conversation, String> {
    state.client.create_dm(&user_id).await.map_err(error_text)
}

#[tauri::command]
async fn update_server(
    state: tauri::State<'_, AppState>,
    server_id: String,
    patch: ServerUpdate,
) -> Result<Server, String> {
    state
        .client
        .update_server(&server_id, patch)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn upload_server_icon(
    state: tauri::State<'_, AppState>,
    server_id: String,
    path: String,
) -> Result<String, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|error| error.to_string())?;
    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("icon.png")
        .to_string();
    let content_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    state
        .client
        .upload_server_icon(&server_id, &file_name, &content_type, bytes)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_channel(
    state: tauri::State<'_, AppState>,
    server_id: String,
    options: CreateChannel,
) -> Result<Channel, String> {
    state
        .client
        .create_channel(&server_id, options)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_category(
    state: tauri::State<'_, AppState>,
    server_id: String,
    name: String,
) -> Result<Category, String> {
    state
        .client
        .create_category(&server_id, &name)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_channel(
    state: tauri::State<'_, AppState>,
    server_id: String,
    channel_id: String,
    patch: ChannelUpdate,
) -> Result<Channel, String> {
    state
        .client
        .update_channel(&server_id, &channel_id, patch)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_channel_positions(
    state: tauri::State<'_, AppState>,
    server_id: String,
    channels: Value,
) -> Result<(), String> {
    state
        .client
        .update_channel_positions(&server_id, channels)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn delete_channel(
    state: tauri::State<'_, AppState>,
    server_id: String,
    channel_id: String,
) -> Result<(), String> {
    state
        .client
        .delete_channel(&server_id, &channel_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_roles(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<RolesResponse, String> {
    state
        .client
        .fetch_roles(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_roles(
    state: tauri::State<'_, AppState>,
    server_id: String,
    batch: Value,
) -> Result<Value, String> {
    state
        .client
        .update_roles(&server_id, batch)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn upload_role_icon(
    state: tauri::State<'_, AppState>,
    server_id: String,
    role_id: String,
    path: String,
) -> Result<Value, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Role icons must be 2 MB or smaller.".to_string());
    }
    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("role-icon.png")
        .to_string();
    let content_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    if !matches!(
        content_type.as_str(),
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    ) {
        return Err("Choose a PNG, JPG, GIF, or WebP image.".to_string());
    }
    state
        .client
        .upload_role_icon(&server_id, &role_id, &file_name, &content_type, bytes)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_server_invites(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Value, String> {
    state
        .client
        .fetch_server_invites(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn delete_server_invite(
    state: tauri::State<'_, AppState>,
    server_id: String,
    code: String,
) -> Result<(), String> {
    state
        .client
        .delete_server_invite(&server_id, &code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn kick_member(
    state: tauri::State<'_, AppState>,
    server_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    state
        .client
        .kick_member(&server_id, &user_id, reason)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_bans(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Value, String> {
    state
        .client
        .fetch_bans(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn ban_member(
    state: tauri::State<'_, AppState>,
    server_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    state
        .client
        .ban_member(&server_id, &user_id, reason)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn unban_member(
    state: tauri::State<'_, AppState>,
    server_id: String,
    user_id: String,
) -> Result<(), String> {
    state
        .client
        .unban_member(&server_id, &user_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_audit_log(
    state: tauri::State<'_, AppState>,
    server_id: String,
    limit: Option<u32>,
    before: Option<String>,
) -> Result<Value, String> {
    state
        .client
        .fetch_audit_log(&server_id, limit.unwrap_or(50), before)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_lifts(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Value, String> {
    state
        .client
        .fetch_lifts(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_lift_supporters(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Value, String> {
    state
        .client
        .fetch_lift_supporters(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_saved_media(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Value, String> {
    state
        .client
        .fetch_saved_media(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_category(
    state: tauri::State<'_, AppState>,
    server_id: String,
    category_id: String,
    patch: Value,
) -> Result<Category, String> {
    state
        .client
        .update_category(&server_id, &category_id, patch)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn delete_category(
    state: tauri::State<'_, AppState>,
    server_id: String,
    category_id: String,
) -> Result<(), String> {
    state
        .client
        .delete_category(&server_id, &category_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_server_permissions(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<String, String> {
    state
        .client
        .fetch_server_permissions(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_server_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ServerSettingsBundle, String> {
    state
        .client
        .fetch_server_settings()
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_channel_permissions(
    state: tauri::State<'_, AppState>,
    server_id: String,
    channel_id: String,
) -> Result<String, String> {
    state
        .client
        .fetch_channel_permissions(&server_id, &channel_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_unread(
    state: tauri::State<'_, AppState>,
) -> Result<UnreadSummary, String> {
    state
        .client
        .fetch_unread_summary()
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_messages(
    state: tauri::State<'_, AppState>,
    target: Target,
    before: Option<String>,
    limit: Option<u32>,
) -> Result<MessagePage, String> {
    state
        .client
        .fetch_messages(
            &target,
            FetchMessagesOptions {
                limit,
                before,
                after: None,
                around: None,
            },
        )
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_messages_around(
    state: tauri::State<'_, AppState>,
    target: Target,
    around: String,
    limit: Option<u32>,
) -> Result<MessagePage, String> {
    state
        .client
        .fetch_messages(
            &target,
            FetchMessagesOptions {
                limit,
                before: None,
                after: None,
                around: Some(around),
            },
        )
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, AppState>,
    target: Target,
    content: String,
    attachments: Option<Vec<Attachment>>,
    reply_to: Option<String>,
) -> Result<Message, String> {
    state
        .client
        .send_message(
            &target,
            content,
            attachments.unwrap_or_default(),
            reply_to,
        )
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn edit_message(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
    content: String,
) -> Result<(), String> {
    state
        .client
        .edit_message(&target, &message_id, content)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn delete_message(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
) -> Result<(), String> {
    state
        .client
        .delete_message(&target, &message_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn send_typing(state: tauri::State<'_, AppState>, target: Target) -> Result<(), String> {
    state.client.send_typing(&target).await.map_err(error_text)
}

#[tauri::command]
async fn stop_typing(state: tauri::State<'_, AppState>, target: Target) -> Result<(), String> {
    state.client.stop_typing(&target).await.map_err(error_text)
}

#[tauri::command]
async fn mark_read(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
) -> Result<(), String> {
    state
        .client
        .mark_read(&target, &message_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn react(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    state
        .client
        .react(&target, &message_id, &emoji)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn unreact(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    state
        .client
        .unreact(&target, &message_id, &emoji)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_members(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<Vec<Member>, String> {
    state
        .client
        .fetch_all_members(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_member_snapshot(
    state: tauri::State<'_, AppState>,
    server_id: String,
    channel_id: String,
) -> Result<MemberListSnapshot, String> {
    state
        .client
        .fetch_member_list_snapshot(&server_id, &channel_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn set_member_roles(
    state: tauri::State<'_, AppState>,
    server_id: String,
    user_id: String,
    add: Vec<String>,
    remove: Vec<String>,
) -> Result<Member, String> {
    state
        .client
        .set_member_roles(&server_id, &user_id, add, remove)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn accept_invite(
    state: tauri::State<'_, AppState>,
    code: String,
) -> Result<AcceptInviteResult, String> {
    state
        .client
        .accept_server_invite(&code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_server(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<CreateServerResult, String> {
    state.client.create_server(&name).await.map_err(error_text)
}

#[tauri::command]
async fn mark_server_read(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<(), String> {
    state
        .client
        .mark_server_read(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_server_user_settings(
    state: tauri::State<'_, AppState>,
    server_id: String,
    patch: Value,
) -> Result<(), String> {
    state
        .client
        .update_server_user_settings(&server_id, patch)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn update_server_positions(
    state: tauri::State<'_, AppState>,
    servers: Value,
) -> Result<(), String> {
    state
        .client
        .update_server_positions(servers)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn leave_server(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Result<(), String> {
    state
        .client
        .leave_server(&server_id)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_server_invite(
    state: tauri::State<'_, AppState>,
    server_id: String,
    options: Value,
) -> Result<Invite, String> {
    state
        .client
        .create_server_invite(&server_id, options)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn block_user(state: tauri::State<'_, AppState>, user_id: String) -> Result<(), String> {
    state.client.block_user(&user_id).await.map_err(error_text)
}

#[tauri::command]
async fn unblock_user(state: tauri::State<'_, AppState>, user_id: String) -> Result<(), String> {
    state.client.unblock_user(&user_id).await.map_err(error_text)
}

#[tauri::command]
async fn report_user(
    state: tauri::State<'_, AppState>,
    user_id: String,
    category: String,
    additional_info: String,
) -> Result<(), String> {
    state
        .client
        .report_user(&user_id, &category, &additional_info)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_sessions(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    state.client.fetch_sessions().await.map_err(error_text)
}

#[tauri::command]
async fn revoke_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
    code: Option<String>,
) -> Result<(), String> {
    state
        .client
        .revoke_session(&session_id, code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn revoke_all_sessions(
    state: tauri::State<'_, AppState>,
    code: Option<String>,
) -> Result<(), String> {
    state
        .client
        .revoke_all_sessions(code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_security_events(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    state
        .client
        .fetch_security_events()
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn clear_security_events(
    state: tauri::State<'_, AppState>,
    code: Option<String>,
) -> Result<(), String> {
    state
        .client
        .clear_security_events(code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn fetch_2fa_status(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    state.client.fetch_2fa_status().await.map_err(error_text)
}

#[tauri::command]
async fn setup_2fa(
    state: tauri::State<'_, AppState>,
    password: String,
) -> Result<Value, String> {
    state.client.setup_2fa(&password).await.map_err(error_text)
}

#[tauri::command]
async fn confirm_2fa(state: tauri::State<'_, AppState>, code: String) -> Result<Value, String> {
    state.client.confirm_2fa(&code).await.map_err(error_text)
}

#[tauri::command]
async fn disable_2fa(
    state: tauri::State<'_, AppState>,
    password: String,
    code: Option<String>,
    backup_code: Option<String>,
) -> Result<Value, String> {
    state
        .client
        .disable_2fa(&password, code, backup_code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_backup_codes(
    state: tauri::State<'_, AppState>,
    password: String,
    code: String,
) -> Result<Value, String> {
    state
        .client
        .create_backup_codes(&password, &code)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn upload_attachment(
    state: tauri::State<'_, AppState>,
    target: Target,
    path: String,
) -> Result<Vec<Attachment>, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|error| error.to_string())?;
    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_string();
    let content_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    state
        .client
        .upload_attachment(&target, &file_name, &content_type, bytes)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn set_status(state: tauri::State<'_, AppState>, status: String) -> Result<(), String> {
    state.client.set_status(&status).await.map_err(error_text)
}

#[tauri::command]
async fn fetch_status(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.client.fetch_status().await.map_err(error_text)
}

#[tauri::command]
async fn update_typing_indicator(
    state: tauri::State<'_, AppState>,
    enabled: bool,
    text: Option<String>,
    style: Option<String>,
    emojis: Option<Vec<String>>,
) -> Result<(), String> {
    state
        .client
        .update_typing_indicator(enabled, text.as_deref(), style.as_deref(), emojis)
        .await
        .map_err(error_text)
}

#[tauri::command]
async fn create_link_preview(
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<Value, String> {
    state.client.create_link_preview(&url).await.map_err(error_text)
}

#[tauri::command]
async fn update_message_embeds(
    state: tauri::State<'_, AppState>,
    target: Target,
    message_id: String,
    tokens: Vec<String>,
) -> Result<Value, String> {
    state
        .client
        .update_message_embeds(&target, &message_id, tokens)
        .await
        .map_err(error_text)
}

#[tauri::command]
fn is_connected(state: tauri::State<'_, AppState>) -> bool {
    state.client.is_connected()
}

#[tauri::command]
fn notify_native(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(windows)]
fn register_notification_identity() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Classes\\AppUserModelId\\dev.flackle.desktop";
    if let Ok((key, _)) = hkcu.create_subkey(path) {
        let _ = key.set_value("DisplayName", &"Flackle Desktop");
        let _ = key.set_value("IconUri", &executable.to_string_lossy().to_string());
        let _ = key.set_value("ShowInSettings", &1u32);
    }
}

#[cfg(not(windows))]
fn register_notification_identity() {}

pub fn run() {
    register_notification_identity();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let _ = std::fs::create_dir_all(&data_dir);

            let client = Client::new();
            let session_dir = data_dir.clone();
            client.set_token_saver(Arc::new(move |tokens| {
                let _ = save_tokens(&session_dir, tokens);
            }));
            let mut events = client.subscribe();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(event) => {
                            let _ = handle.emit("flackle://gateway", &event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            });

            let open_item = MenuItem::with_id(app, "open", "Open Flackle", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("Flackle")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            app.manage(AppState { client, data_dir });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            login,
            login_mfa,
            restore,
            logout,
            bootstrap,
            fetch_servers,
            fetch_server,
            fetch_channels,
            fetch_conversations,
            fetch_friends,
            fetch_blocked,
            fetch_friend_requests,
            add_friend,
            accept_friend_request,
            reject_friend_request,
            cancel_friend_request,
            remove_friend,
            create_dm,
            update_server,
            upload_server_icon,
            create_channel,
            create_category,
            update_channel,
            update_channel_positions,
            delete_channel,
            fetch_roles,
            update_roles,
            upload_role_icon,
            fetch_server_invites,
            delete_server_invite,
            kick_member,
            fetch_bans,
            ban_member,
            unban_member,
            fetch_audit_log,
            fetch_lifts,
            fetch_lift_supporters,
            fetch_saved_media,
            update_category,
            delete_category,
            fetch_server_permissions,
            fetch_server_settings,
            fetch_channel_permissions,
            fetch_unread,
            fetch_messages,
            fetch_messages_around,
            send_message,
            edit_message,
            delete_message,
            send_typing,
            stop_typing,
            mark_read,
            react,
            unreact,
            fetch_members,
            fetch_member_snapshot,
            set_member_roles,
            accept_invite,
            create_server,
            mark_server_read,
            update_server_user_settings,
            update_server_positions,
            leave_server,
            create_server_invite,
            block_user,
            unblock_user,
            report_user,
            fetch_sessions,
            revoke_session,
            revoke_all_sessions,
            fetch_security_events,
            clear_security_events,
            fetch_2fa_status,
            setup_2fa,
            confirm_2fa,
            disable_2fa,
            create_backup_codes,
            upload_attachment,
            set_status,
            fetch_status,
            update_typing_indicator,
            create_link_preview,
            update_message_embeds,
            is_connected,
            notify_native
        ])
        .run(tauri::generate_context!())
        .expect("error while running Flackle Desktop");
}
