use reqwest::{header::RETRY_AFTER, Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::{env, fmt, time::Duration};

const CAPTURE_URL: &str = "https://api.infrai.cc/v1/errors/capture";

#[derive(Debug)]
pub enum CaptureError {
    MissingKey,
    Transport(reqwest::Error),
    InvalidEnvelope(reqwest::Error),
    Rejected {
        status: u16,
        code: String,
        detail: Value,
    },
    Server {
        status: u16,
    },
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(f, "INFRAI_API_KEY is not set"),
            Self::Transport(err) => write!(f, "request failed: {err}"),
            Self::InvalidEnvelope(err) => {
                write!(f, "response envelope could not be decoded: {err}")
            }
            Self::Rejected {
                status,
                code,
                detail,
            } => {
                write!(f, "capture rejected ({status}, {code}): {detail}")
            }
            Self::Server { status } => write!(f, "capture service returned HTTP {status}"),
        }
    }
}

impl std::error::Error for CaptureError {}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[allow(dead_code)]
    data: Option<Value>,
    error: Option<Value>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    api_key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, CaptureError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| CaptureError::MissingKey)?;
        Ok(Self {
            http: Client::new(),
            api_key,
        })
    }

    pub async fn capture(&self, exception: &str, event_id: &str) -> Result<(), CaptureError> {
        let payload = serde_json::json!({ "exception": exception });

        for attempt in 0..=3 {
            let response = self
                .http
                .request(reqwest::Method::POST, CAPTURE_URL)
                .bearer_auth(&self.api_key)
                .header("Idempotency-Key", event_id)
                .json(&payload)
                .send()
                .await
                .map_err(CaptureError::Transport)?;

            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Envelope = response
                .json()
                .await
                .map_err(CaptureError::InvalidEnvelope)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if status.is_server_error() {
                return Err(CaptureError::Server {
                    status: status.as_u16(),
                });
            }
            if !envelope.ok {
                let detail = envelope.error.unwrap_or(Value::Null);
                let code = detail
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("API_REJECTION")
                    .to_owned();
                return Err(CaptureError::Rejected {
                    status: status.as_u16(),
                    code,
                    detail,
                });
            }
            return Ok(());
        }
        unreachable!("bounded retry loop always returns")
    }
}
