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
    #[serde(rename = "@wfm|event/status/set")]
    StatusSet { payload: StatusSetEventPayload },
    #[serde(rename = "@wfm|cmd/auth/signIn:ok")]
    AuthOk,
    #[serde(rename = "@wfm|cmd/auth/signIn:error")]
    AuthError,
    #[serde(other)]
    Unknown,
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
            IncomingEvent::Unknown
        ));
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|event/status/set","payload":{"status":"ingame","statusSetAt":"2026-09-07T18:55:39Z","statusUntil":null,"activity":null}}"#)
                .unwrap(),
            IncomingEvent::StatusSet { payload } if payload.status == UserStatus::Ingame
        ));
        assert!(matches!(
            parse_event(r#"{"route":"@wfm|event/orders/new","payload":{}}"#).unwrap(),
            IncomingEvent::Unknown
        ));
    }
}
