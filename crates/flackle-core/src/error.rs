use serde_json::Value;

/// Errors surfaced by the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(String),
    #[error("request timed out")]
    Timeout,
    #[error("not authenticated")]
    NotAuthenticated,
    #[error("session expired; please log in again")]
    SessionExpired,
    #[error("gateway is not connected")]
    GatewayClosed,
    #[error("timed out waiting for message acknowledgement")]
    AckTimeout,
    #[error("API error {status}: {message}")]
    Api {
        status: u16,
        code: Option<String>,
        message: String,
        body: Value,
    },
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn api(status: u16, body: Value) -> Self {
        let code = body
            .get("code")
            .and_then(Value::as_str)
            .map(str::to_string);
        let message = body
            .get("error")
            .or_else(|| body.get("message"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}"));
        Self::Api {
            status,
            code,
            message,
            body,
        }
    }

    pub fn is_csrf(&self) -> bool {
        matches!(self, Self::Api { code: Some(code), .. } if code == "csrf_invalid")
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            Self::Timeout
        } else {
            Self::Network(value.to_string())
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Other(format!("invalid response: {value}"))
    }
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
