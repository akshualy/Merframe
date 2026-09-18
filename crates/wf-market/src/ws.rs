use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::error::Result;
use crate::models::{Activity, UserStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Envelope<P> {
    pub route: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<P>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthSignInPayload {
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusSetPayload {
    pub status: UserStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusSetEventPayload {
    pub status: UserStatus,
    #[serde(rename = "statusSetAt")]
    pub status_set_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "statusUntil")]
    pub status_until: Option<chrono::DateTime<chrono::Utc>>,
    pub activity: Option<Activity>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "route")]
pub enum IncomingEvent {
    #[serde(rename = "@wfm|event/status/set")]
    StatusSet {
        payload: StatusSetEventPayload,
        meta: Option<EnvelopeMeta>,
    },
    #[serde(rename = "@wfm|event/reports/online")]
    OnlineReport { payload: OnlineReportPayload },
    #[serde(rename = "@wfm|cmd/auth/signIn:ok")]
    AuthOk,
    #[serde(rename = "@wfm|cmd/auth/signIn:error")]
    AuthError,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OnlineReportPayload {
    pub connections: u64,
    #[serde(rename = "authorizedUsers")]
    pub authorized_users: u64,
}

pub fn parse_event(json: &str) -> Result<IncomingEvent> {
    Ok(serde_json::from_str(json)?)
}

pub struct MarketSocket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl MarketSocket {
    pub async fn connect() -> Result<Self> {
        let mut request = "wss://ws.warframe.market/socket".into_client_request()?;
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("wfm"));
        let (stream, _response) = connect_async(request).await?;
        Ok(Self { stream })
    }

    pub async fn authenticate(&mut self, token: String) -> Result<()> {
        let envelope = Envelope {
            route: "@wfm|cmd/auth/signIn".to_owned(),
            id: None,
            payload: Some(AuthSignInPayload { token }),
        };
        let text = serde_json::to_string(&envelope)?;
        self.stream.send(Message::text(text)).await?;
        Ok(())
    }

    pub async fn set_status(&mut self, payload: StatusSetPayload) -> Result<()> {
        let envelope = Envelope {
            route: "@wfm|cmd/status/set".to_owned(),
            id: None,
            payload: Some(payload),
        };
        let text = serde_json::to_string(&envelope)?;
        self.stream.send(Message::text(text)).await?;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<Option<IncomingEvent>> {
        while let Some(message) = self.stream.next().await {
            match message? {
                Message::Text(text) => return Ok(Some(parse_event(text.as_str())?)),
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_frames() {
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|cmd/auth/signIn:ok"}"#).unwrap(),
            IncomingEvent::AuthOk
        ));
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|cmd/auth/signIn:error","payload":"invalid token"}"#)
                .unwrap(),
            IncomingEvent::AuthError
        ));
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|event/reports/online","payload":{"connections":812,"authorizedUsers":97}}"#)
                .unwrap(),
            IncomingEvent::OnlineReport { .. }
        ));
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|event/orders/new","payload":{}}"#).unwrap(),
            IncomingEvent::Unknown
        ));
    }
}
