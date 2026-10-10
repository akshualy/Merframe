use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Runtime};
use tracing::{debug, info, warn};
use wf_market::{MarketError, Platform, Session};

use super::{
    MARKET_SIGNED_OUT_POLL, market_refresh, market_snapshot, refresh_failed, sign_out,
    signed_in_slug,
};
use crate::market::MarketSnapshot;
use crate::runtime::{AppEvent, emit};
use crate::settings::{self, MarketAccount};
use crate::state::{AppState, lock, read, write};

pub(crate) async fn market_task<R: Runtime>(app: AppHandle<R>, state: Arc<AppState>) {
    let mut failures: u32 = 0;
    let mut verified_at: Option<Instant> = None;
    let mut emitted: Option<MarketSnapshot> = None;
    loop {
        if !state.market().has_token() {
            failures = 0;
            verified_at = None;
            tokio::time::sleep(MARKET_SIGNED_OUT_POLL).await;
            continue;
        }

        let since_activity = lock(&state.market_activity).map(|touched| touched.elapsed());
        let interval = poll_interval(read(&state.settings).market_poll_interval(), since_activity);

        let checked_recently = verified_at.is_some_and(|at| at.elapsed() < Duration::from_hours(5));
        let slug = match signed_in_slug(&state) {
            Some(slug) if checked_recently => Some(slug),
            known => {
                let checked = checked_session(&app, &state, known.is_some()).await;
                verified_at = checked.as_ref().map(|_| Instant::now());
                checked
            }
        };
        let Some(slug) = slug else {
            tokio::time::sleep(interval).await;
            continue;
        };

        refresh_unread(&app, &state).await;

        let refresh = market_refresh(&state, &slug).await;
        if refresh.is_complete() {
            failures = 0;
        } else {
            failures = failures.saturating_add(1);
        }

        if !refresh.is_empty() {
            let snapshot = market_snapshot(&state, refresh).await;
            debug!(
                orders = snapshot.orders.as_ref().map(|orders| orders.rows.len()),
                auctions = snapshot.auctions.as_ref().map(Vec::len),
                "Market listings refreshed"
            );

            if emitted
                .as_ref()
                .is_none_or(|last| !last.same_listings(&snapshot))
            {
                emit(&app, AppEvent::MarketUpdated(snapshot.clone()));
                emitted = Some(snapshot);
            }
        }

        tokio::time::sleep(poll_backoff(interval, failures)).await;
    }
}

fn poll_interval(idle: Duration, since_activity: Option<Duration>) -> Duration {
    if since_activity.is_some_and(|elapsed| elapsed < Duration::from_mins(5)) {
        Duration::from_secs(60)
    } else {
        idle
    }
}

fn poll_backoff(interval: Duration, failures: u32) -> Duration {
    interval * (1 << failures.min(3))
}

async fn checked_session<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    was_signed_in: bool,
) -> Option<String> {
    let session = match state.market().me().await {
        Ok(session) => session,
        Err(MarketError::Unauthorized) => {
            info!("Warframe.market rejected the stored token");
            sign_out(app, state);
            return None;
        }
        Err(error) => {
            warn!(error = %error.brief(), "Warframe.market session check failed");
            return None;
        }
    };

    if let Some(token) = &session.rotated_token {
        debug!("Warframe.market issued a new token");
        if let Err(error) = settings::set_token(app, Some(token)) {
            warn!(%error, "Rotated token save failed");
        }
        *write(&state.market) = Arc::new(
            wf_market::Client::new(state.http.clone(), Platform::Pc).with_token(token.clone()),
        );
    }

    let account = account_of(&session);
    if !was_signed_in {
        info!(account = account.ingame_name, "Warframe.market signed in");
        if let Err(error) = settings::set_account(app, Some(&account)) {
            warn!(%error, "Market account save failed");
        }
        write(&state.status).market_account = Some(account.clone());
        emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    }

    Some(account.slug)
}

fn account_of(session: &Session) -> MarketAccount {
    MarketAccount {
        ingame_name: session.user.ingame_name.clone(),
        slug: session.user.slug.clone(),
        tier: session.user.tier.clone(),
        mastery_rank: session.user.mastery_rank,
    }
}

async fn refresh_unread<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let chats = match state.market().chats().await {
        Ok(chats) => chats,
        Err(error) => {
            refresh_failed("messages", &error);
            return;
        }
    };

    let unread = chats.iter().map(|chat| chat.unread_count).sum();
    if read(&state.status).market_unread != unread {
        write(&state.status).market_unread = unread;
        emit(app, AppEvent::StatusUpdated(state.status_snapshot()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_page_polls_every_minute() {
        let idle = Duration::from_secs(300);
        assert_eq!(poll_interval(idle, None), idle);
        assert_eq!(
            poll_interval(idle, Some(Duration::from_secs(10))),
            Duration::from_secs(60)
        );
        assert_eq!(
            poll_interval(idle, Some(Duration::from_secs(299))),
            Duration::from_secs(60)
        );
        assert_eq!(poll_interval(idle, Some(Duration::from_secs(301))), idle);
    }

    #[test]
    fn no_failures_no_backoff() {
        let interval = Duration::from_secs(300);
        assert_eq!(poll_backoff(interval, 0), interval);
    }

    #[test]
    fn poll_backoff_doubles_to_eight() {
        let interval = Duration::from_secs(300);
        assert_eq!(poll_backoff(interval, 1), interval * 2);
        assert_eq!(poll_backoff(interval, 2), interval * 4);
        assert_eq!(poll_backoff(interval, 3), interval * 8);
        assert_eq!(poll_backoff(interval, 9), interval * 8);
        assert_eq!(poll_backoff(interval, 9).as_secs(), 2400);
    }
}
