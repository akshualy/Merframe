use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::error::Result;
use crate::models::UserStatus;

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusSetEventPayload {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "route")]
pub enum IncomingEvent {
    #[serde(rename = "@wfm|event/status/set", alias = "@wfm|cmd/status/set:ok")]
    StatusSet { payload: StatusSetEventPayload },
    #[serde(rename = "@wfm|cmd/auth/signIn:ok")]
    AuthOk,
    #[serde(rename = "@wfm|cmd/auth/signIn:error")]
    AuthError,
    #[serde(other)]
    Unknown,
}

pub struct MarketSocket {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl MarketSocket {
    pub async fn connect(user_agent: &'static str) -> Result<Self> {
        let mut request = "wss://ws.warframe.market/socket".into_client_request()?;
        let headers = request.headers_mut();
        headers.insert("Sec-WebSocket-Protocol", HeaderValue::from_static("wfm"));
        headers.insert("User-Agent", HeaderValue::from_static(user_agent));
        let (stream, _response) = connect_async(request).await?;
        Ok(Self { stream })
    }

    pub async fn authenticate(&mut self, token: String) -> Result<()> {
        self.send("@wfm|cmd/auth/signIn", AuthSignInPayload { token })
            .await
    }

    pub async fn set_status(&mut self, payload: StatusSetPayload) -> Result<()> {
        self.send("@wfm|cmd/status/set", payload).await
    }

    async fn send<P: Serialize>(&mut self, route: &str, payload: P) -> Result<()> {
        let envelope = Envelope {
            route: route.to_owned(),
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
                Message::Text(text) => {
                    return Ok(Some(serde_json::from_str::<IncomingEvent>(text.as_str())?));
                }
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
            serde_json::from_str::<IncomingEvent>(r#"{"route":"@wfm|cmd/auth/signIn:ok"}"#)
                .unwrap(),
            IncomingEvent::AuthOk
        ));
        assert!(matches!(
            serde_json::from_str::<IncomingEvent>(
                r#"{"route":"@wfm|cmd/auth/signIn:error","payload":"invalid token"}"#
            )
            .unwrap(),
            IncomingEvent::AuthError
        ));
        assert!(matches!(
            serde_json::from_str::<IncomingEvent>(r#"{"route":"@wfm|event/reports/online","payload":{"connections":812,"authorizedUsers":97}}"#)
                .unwrap(),
            IncomingEvent::Unknown
        ));
        assert!(matches!(
            serde_json::from_str::<IncomingEvent>(r#"{"route":"@wfm|event/status/set","payload":{"status":"ingame","statusSetAt":"2026-09-07T18:55:39Z","statusUntil":null,"activity":null}}"#)
                .unwrap(),
            IncomingEvent::StatusSet { payload } if payload.status == UserStatus::Ingame
        ));
        assert!(matches!(
            serde_json::from_str::<IncomingEvent>(r#"{"route":"@wfm|cmd/status/set:ok","payload":{"status":"online","statusSetAt":"2026-10-27T15:43:09Z"},"meta":{"stream":"status:0","revision":2}}"#)
                .unwrap(),
            IncomingEvent::StatusSet { payload } if payload.status == UserStatus::Online
        ));
        assert!(matches!(
            serde_json::from_str::<IncomingEvent>(
                r#"{"route":"@wfm|event/orders/new","payload":{}}"#
            )
            .unwrap(),
            IncomingEvent::Unknown
        ));
    }
}
