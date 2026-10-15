use std::{convert::Infallible, env};

use axum::{
    Router,
    extract::State,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use serde::Serialize;
use tokio::{
    net::TcpListener,
    sync::{broadcast, watch},
};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::runtime::AppEvent;

pub const SSE_DEFAULT_PORT: &str = "28005";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Default)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub enum HttpServerStatus {
    Starting,
    Running,
    Stopped,
    Failed,
    #[default]
    None,
}

pub fn spawn_axum_app(
    app_event_tx: broadcast::Sender<AppEvent>,
    status_tx: watch::Sender<HttpServerStatus>,
    mut should_run_rx: watch::Receiver<bool>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = should_run_rx.wait_for(|&should_run| should_run).await;

            status_tx.send_replace(HttpServerStatus::Starting);

            let router = Router::new()
                .route("/events", axum::routing::get(events_sse_handler))
                // we pass a tx here because the broadcast receiver doesn't impl clone
                // in the handler we will subscribe to the stream via this sender
                .with_state(app_event_tx.clone());

            let port = env::var("SSE_PORT").unwrap_or_else(|_| SSE_DEFAULT_PORT.to_owned());

            let listener = match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => listener,
                Err(err) => {
                    tracing::error!("Failed to bind SSE server: {err}");
                    status_tx.send_replace(HttpServerStatus::Failed);
                    return;
                }
            };

            status_tx.send_replace(HttpServerStatus::Running);

            let mut should_run_rx_clone = should_run_rx.clone();
            if let Err(err) = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = should_run_rx_clone
                        .wait_for(|&should_run| !should_run)
                        .await;
                })
                .await
            {
                tracing::error!("SSE server failed: {err}");
                status_tx.send_replace(HttpServerStatus::Failed);
                return;
            }

            status_tx.send_replace(HttpServerStatus::Stopped);
        }
    });
}

async fn events_sse_handler(
    State(tx): State<broadcast::Sender<AppEvent>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe()).filter_map(|result| match result {
        Ok(app_event) => serde_json::to_string(&app_event)
            .ok()
            .map(|data| Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
