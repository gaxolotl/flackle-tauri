use crate::error::{Error, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, ORIGIN, REFERER, USER_AGENT};
use reqwest::{Method, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub const DEFAULT_BASE_URL: &str = "https://flackle.com";
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/155.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub csrf_token: Option<String>,
}

struct LimiterState {
    next: Instant,
    paused_until: Option<Instant>,
}

struct RateLimiter {
    interval: Duration,
    state: Mutex<LimiterState>,
}

impl RateLimiter {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: Mutex::new(LimiterState {
                next: Instant::now(),
                paused_until: None,
            }),
        }
    }

    async fn acquire(&self) {
        let wait = {
            let mut state = self.state.lock().unwrap();
            let now = Instant::now();
            let start = state
                .next
                .max(state.paused_until.unwrap_or(now))
                .max(now);
            state.next = start + self.interval;
            start.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            sleep(wait).await;
        }
    }

    fn block_for(&self, duration: Duration) {
        let mut state = self.state.lock().unwrap();
        let until = Instant::now() + duration;
        state.paused_until = Some(match state.paused_until {
            Some(current) => current.max(until),
            None => until,
        });
    }
}

/// A cookie jar that keeps the handful of Flackle session cookies and can
/// expose them for encrypted storage.
#[derive(Clone, Default)]
struct CookieJar {
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl CookieJar {
    fn get(&self, name: &str) -> Option<String> {
        self.values.lock().unwrap().get(name).cloned()
    }

    fn set(&self, name: &str, value: impl Into<String>) {
        self.values
            .lock()
            .unwrap()
            .insert(name.to_string(), value.into());
    }

    fn remove(&self, name: &str) {
        self.values.lock().unwrap().remove(name);
    }

    fn clear(&self) {
        self.values.lock().unwrap().clear();
    }

    fn header(&self) -> Option<String> {
        let values = self.values.lock().unwrap();
        if values.is_empty() {
            return None;
        }
        Some(
            values
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    fn absorb(&self, headers: &HeaderMap) {
        let mut jar = self.values.lock().unwrap();
        for value in headers.get_all(reqwest::header::SET_COOKIE).iter() {
            let Ok(raw) = value.to_str() else { continue };
            let first = raw.split(';').next().unwrap_or_default();
            let Some((name, value)) = first.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let lower = raw.to_ascii_lowercase();
            if lower.contains("max-age=0") || lower.contains("expires=thu, 01 jan 1970") {
                jar.remove(name);
            } else {
                jar.insert(name.to_string(), value.trim().trim_matches('"').to_string());
            }
        }
    }
}

#[derive(Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
    user_agent: String,
    jar: CookieJar,
    limiter: Arc<RateLimiter>,
    access_token: Arc<RwLock<Option<String>>>,
    token_saver: Arc<Mutex<Option<Arc<dyn Fn(&Tokens) + Send + Sync>>>>,
    max_retries: u32,
    timeout: Duration,
}

impl RestClient {
    pub fn new() -> Self {
        Self::with_options(DEFAULT_BASE_URL, DEFAULT_USER_AGENT, 50, 30_000, 3)
    }

    pub fn with_options(
        base_url: &str,
        user_agent: &str,
        request_interval_ms: u64,
        timeout_ms: u64,
        max_retries: u32,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            user_agent: user_agent.to_string(),
            jar: CookieJar::default(),
            limiter: Arc::new(RateLimiter::new(Duration::from_millis(
                request_interval_ms.max(1),
            ))),
            access_token: Arc::new(RwLock::new(None)),
            token_saver: Arc::new(Mutex::new(None)),
            max_retries,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn has_session(&self) -> bool {
        self.access_token.read().unwrap().is_some()
            || self.jar.get("access_token").is_some()
            || self.jar.get("refresh_token").is_some()
    }

    pub fn csrf_token(&self) -> Option<String> {
        self.jar.get("__Host-csrf")
    }

    pub fn tokens(&self) -> Tokens {
        Tokens {
            access_token: self.access_token.read().unwrap().clone(),
            refresh_token: self.jar.get("refresh_token"),
            csrf_token: self.jar.get("__Host-csrf"),
        }
    }

    pub fn set_tokens(&self, tokens: &Tokens) {
        if let Some(token) = &tokens.access_token {
            *self.access_token.write().unwrap() = Some(token.clone());
            self.jar.set("access_token", token.clone());
        }
        if let Some(token) = &tokens.refresh_token {
            self.jar.set("refresh_token", token.clone());
        }
        if let Some(token) = &tokens.csrf_token {
            self.jar.set("__Host-csrf", token.clone());
        }
    }

    pub fn set_token_saver(&self, saver: Arc<dyn Fn(&Tokens) + Send + Sync>) {
        *self.token_saver.lock().unwrap() = Some(saver);
    }

    pub(crate) fn save_tokens(&self) {
        let saver = self.token_saver.lock().unwrap().clone();
        if let Some(saver) = saver {
            saver(&self.tokens());
        }
    }

    pub fn clear_session(&self) {
        *self.access_token.write().unwrap() = None;
        self.jar.clear();
    }

    fn absolute(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    fn headers(&self, method: &Method, with_cookies: bool, with_auth: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-GB,en;q=0.9"),
        );
        if let Ok(value) = HeaderValue::from_str(&self.user_agent) {
            headers.insert(USER_AGENT, value);
        }
        headers.insert(
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        );
        if method != Method::GET {
            if let Ok(value) = HeaderValue::from_str(&self.base_url) {
                headers.insert(ORIGIN, value);
            }
        }
        if let Ok(value) = HeaderValue::from_str(&format!("{}/app", self.base_url)) {
            headers.insert(REFERER, value);
        }
        if with_cookies {
            if let Some(cookie) = self.jar.header() {
                if let Ok(value) = HeaderValue::from_str(&cookie) {
                    headers.insert(reqwest::header::COOKIE, value);
                }
            }
        }
        if with_auth {
            if let Some(token) = self.access_token.read().unwrap().clone() {
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}")) {
                    headers.insert(reqwest::header::AUTHORIZATION, value);
                }
            }
        }
        if let Some(csrf) = self.csrf_token() {
            if let Ok(value) = HeaderValue::from_str(&csrf) {
                headers.insert(HeaderName::from_static("x-csrf-token"), value);
            }
        }
        headers
    }

    /// Fetches the double-submit CSRF cookie if it is missing. Flackle returns
    /// it on the 401 from `/api/auth/me`.
    pub async fn ensure_csrf(&self) -> Option<String> {
        if let Some(token) = self.csrf_token() {
            return Some(token);
        }
        self.ensure_csrf_force().await
    }

    pub async fn ensure_csrf_force(&self) -> Option<String> {
        self.jar.remove("__Host-csrf");
        self.limiter.acquire().await;
        let url = self.absolute("/api/auth/me");
        let response = self
            .http
            .get(url)
            .headers(self.headers(&Method::GET, true, true))
            .send()
            .await
            .ok()?;
        self.jar.absorb(response.headers());
        drop(response);
        self.csrf_token()
    }

    pub async fn refresh(&self) -> Result<()> {
        let refresh_token = self
            .jar
            .get("refresh_token")
            .ok_or(Error::SessionExpired)?;
        self.jar.set("refresh_token", refresh_token);
        self.limiter.acquire().await;
        let url = self.absolute("/api/auth/refresh");
        let headers = self.headers(&Method::POST, true, false);
        let response = self.http.post(url).headers(headers).send().await?;
        self.jar.absorb(response.headers());
        let status = response.status();
        let value = read_value(response).await;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.clear_session();
            return Err(Error::SessionExpired);
        }
        if !status.is_success() {
            return Err(Error::api(status.as_u16(), value));
        }
        let token = value
            .get("access_token")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| self.jar.get("access_token"))
            .ok_or(Error::SessionExpired)?;
        *self.access_token.write().unwrap() = Some(token);
        self.save_tokens();
        Ok(())
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let value = self
            .request_json(Method::GET, path, query, None, true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        let value = self
            .request_json(Method::POST, path, &[], Some(body), true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn post_anonymous<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<T> {
        let value = self
            .request_json(Method::POST, path, &[], Some(body), false)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn patch<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        let value = self
            .request_json(Method::PATCH, path, &[], Some(body), true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: Value) -> Result<T> {
        let value = self
            .request_json(Method::PUT, path, &[], Some(body), true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self
            .request_json(Method::DELETE, path, &[], None, true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn delete_with_body<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Value,
    ) -> Result<T> {
        let value = self
            .request_json(Method::DELETE, path, &[], Some(body), true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn put_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self
            .request_json(Method::PUT, path, &[], None, true)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    async fn request_json(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
        auth: bool,
    ) -> Result<Value> {
        let mut url = reqwest::Url::parse(&self.absolute(path))
            .map_err(|error| Error::Other(format!("invalid URL: {error}")))?;
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }

        if method != Method::GET && auth {
            self.ensure_csrf().await;
        }

        let mut refreshed = false;
        let mut csrf_retried = false;
        let mut attempt = 0u32;

        loop {
            self.limiter.acquire().await;
            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .headers(self.headers(&method, true, auth));
            if let Some(body) = &body {
                request = request.json(body);
            }
            let response = request.send().await?;
            self.jar.absorb(response.headers());
            let status = response.status();
            let retry_after = parse_retry_after(response.headers());
            let value = read_value(response).await;

            if status == StatusCode::UNAUTHORIZED
                && auth
                && !refreshed
                && self.jar.get("refresh_token").is_some()
            {
                refreshed = true;
                self.refresh().await?;
                continue;
            }

            if status == StatusCode::FORBIDDEN
                && !csrf_retried
                && value.get("code").and_then(Value::as_str) == Some("csrf_invalid")
            {
                csrf_retried = true;
                self.ensure_csrf_force().await;
                continue;
            }

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < self.max_retries {
                attempt += 1;
                let delay = retry_after.unwrap_or_else(|| Duration::from_secs(1 << attempt));
                self.limiter.block_for(delay);
                sleep(delay).await;
                continue;
            }

            if status.is_server_error() && attempt < self.max_retries {
                attempt += 1;
                let delay = retry_after.unwrap_or_else(|| Duration::from_millis(500 * 2u64.pow(attempt)));
                sleep(delay).await;
                continue;
            }

            if !status.is_success() {
                return Err(Error::api(status.as_u16(), value));
            }
            return Ok(value);
        }
    }

    /// Uploads bytes directly to a channel/conversation attachment endpoint.
    pub async fn upload_attachment(
        &self,
        path: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<Value> {
        self.upload_file(path, "file", file_name, content_type, bytes)
            .await
    }

    /// Uploads bytes as multipart form data using an explicit field name.
    pub async fn upload_file(
        &self,
        path: &str,
        field: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<Value> {
        self.ensure_csrf().await;
        self.limiter.acquire().await;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str(content_type)
            .map_err(|error| Error::Other(error.to_string()))?;
        let form = reqwest::multipart::Form::new().part(field.to_string(), part);
        let url = reqwest::Url::parse(&self.absolute(path))
            .map_err(|error| Error::Other(format!("invalid URL: {error}")))?;
        let response = self
            .http
            .post(url)
            .headers(self.headers(&Method::POST, true, true))
            .multipart(form)
            .timeout(self.timeout.max(Duration::from_secs(300)))
            .send()
            .await?;
        self.jar.absorb(response.headers());
        let status = response.status();
        let value = read_value(response).await;
        if !status.is_success() {
            return Err(Error::api(status.as_u16(), value));
        }
        Ok(value)
    }

    /// PATCH a roles batch as multipart form data with one uploaded role icon.
    pub async fn patch_role_icon(
        &self,
        path: &str,
        role_id: &str,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
        changes: Value,
    ) -> Result<Value> {
        self.ensure_csrf().await;
        self.limiter.acquire().await;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str(content_type)
            .map_err(|error| Error::Other(error.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .text("changes", changes.to_string())
            .part(format!("icon_{role_id}"), part);
        let url = reqwest::Url::parse(&self.absolute(path))
            .map_err(|error| Error::Other(format!("invalid URL: {error}")))?;
        let response = self
            .http
            .patch(url)
            .headers(self.headers(&Method::PATCH, true, true))
            .multipart(form)
            .timeout(self.timeout.max(Duration::from_secs(300)))
            .send()
            .await?;
        self.jar.absorb(response.headers());
        let status = response.status();
        let value = read_value(response).await;
        if !status.is_success() {
            return Err(Error::api(status.as_u16(), value));
        }
        Ok(value)
    }

    /// PUT a part to a presigned URL and return its ETag.
    pub async fn put_absolute(&self, url: &str, bytes: Vec<u8>) -> Result<String> {
        let response = self
            .http
            .put(url)
            .body(bytes)
            .timeout(Duration::from_secs(300))
            .send()
            .await?;
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let status = response.status();
        if !status.is_success() {
            let value = read_value(response).await;
            return Err(Error::api(status.as_u16(), value));
        }
        etag.ok_or_else(|| Error::Other("upload part returned no ETag".to_string()))
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

async fn read_value(response: Response) -> Value {
    let status = response.status();
    if status == StatusCode::NO_CONTENT {
        return Value::Null;
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let text = response.text().await.unwrap_or_default();
    if text.is_empty() {
        return Value::Null;
    }
    if content_type.contains("json") {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    } else {
        Value::String(text)
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = raw.parse::<f64>() {
        return Some(Duration::from_secs_f64(seconds.max(0.0)));
    }
    None
}
