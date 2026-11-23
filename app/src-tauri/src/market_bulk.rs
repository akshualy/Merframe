use std::mem::DropGuard;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tokio::sync::watch;
use wf_core::counted;
use wf_market::{MarketError, Order};

use crate::error::{CommandError, CommandResult, cause_chain};
use crate::market;
use crate::runtime::{self, AppEvent};
use crate::state::{AppState, read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketBulkRunState {
    Idle,
    Running,
    Stopping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct MarketBulkProgress {
    pub written: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(tag = "ended", rename_all = "snake_case")]
pub enum MarketBulkOutcome {
    Finished { written: usize },
    Stopped { written: usize },
}

#[derive(Debug)]
pub(crate) struct MarketBulkFailure {
    written: usize,
    error: MarketError,
}

impl From<MarketBulkFailure> for CommandError {
    fn from(failure: MarketBulkFailure) -> Self {
        Self::from(format!(
            "Stopped after {}: {}",
            counted(failure.written, "order"),
            cause_chain(&failure.error)
        ))
    }
}

pub(crate) struct MarketBulkRun<R: Runtime> {
    app: AppHandle<R>,
    state: DropGuard<Arc<AppState>, fn(Arc<AppState>)>,
}

impl<R: Runtime> MarketBulkRun<R> {
    pub(crate) fn start(app: &AppHandle<R>, state: &Arc<AppState>) -> CommandResult<Self> {
        if !claim(&state.market_bulk_run) {
            return Err(CommandError::from(
                "Another change to all your orders is still running",
            ));
        }

        Ok(Self {
            app: app.clone(),
            state: DropGuard::new(Arc::clone(state), |state| {
                state.market_bulk_run.send_replace(MarketBulkRunState::Idle);
            }),
        })
    }

    pub(crate) async fn write_each<T, W>(
        &self,
        targets: Vec<T>,
        write: impl Fn(T) -> W,
        failed: impl Fn(T, &MarketError),
    ) -> CommandResult<MarketBulkOutcome>
    where
        T: Copy,
        W: Future<Output = wf_market::Result<Order>>,
    {
        let pause = read(&self.state.settings).market_bulk_pause();
        let outcome = paced_writes(
            targets,
            pause,
            self.state.market_bulk_run.subscribe(),
            |progress| runtime::emit(&self.app, AppEvent::MarketBulkProgress(progress)),
            write,
            failed,
        )
        .await;
        market::publish_orders(&self.app, &self.state).await;

        outcome.map_err(CommandError::from)
    }
}

fn claim(run: &watch::Sender<MarketBulkRunState>) -> bool {
    transition(run, MarketBulkRunState::Idle, MarketBulkRunState::Running)
}

pub(crate) fn stop(run: &watch::Sender<MarketBulkRunState>) {
    transition(
        run,
        MarketBulkRunState::Running,
        MarketBulkRunState::Stopping,
    );
}

fn transition(
    run: &watch::Sender<MarketBulkRunState>,
    from: MarketBulkRunState,
    to: MarketBulkRunState,
) -> bool {
    run.send_if_modified(|current| {
        let matched = *current == from;
        if matched {
            *current = to;
        }

        matched
    })
}

async fn paced_writes<T, W, O>(
    targets: Vec<T>,
    pause: Duration,
    mut run: watch::Receiver<MarketBulkRunState>,
    progress: impl Fn(MarketBulkProgress),
    write: impl Fn(T) -> W,
    failed: impl Fn(T, &MarketError),
) -> Result<MarketBulkOutcome, MarketBulkFailure>
where
    T: Copy,
    W: Future<Output = wf_market::Result<O>>,
{
    let total = targets.len();
    let mut written = 0;

    for (index, target) in targets.into_iter().enumerate() {
        let wait = if index == 0 { Duration::ZERO } else { pause };
        if stopped_during(wait, &mut run).await {
            return Ok(MarketBulkOutcome::Stopped { written });
        }

        let result = loop {
            let retry_after = match write(target).await {
                Err(MarketError::RateLimited(retry_after)) => retry_after,
                result => break result,
            };
            if stopped_during(retry_after, &mut run).await {
                return Ok(MarketBulkOutcome::Stopped { written });
            }
        };

        if let Err(error) = result {
            failed(target, &error);
            return Err(MarketBulkFailure { written, error });
        }

        written += 1;
        progress(MarketBulkProgress { written, total });
    }

    Ok(MarketBulkOutcome::Finished { written })
}

async fn stopped_during(pause: Duration, run: &mut watch::Receiver<MarketBulkRunState>) -> bool {
    tokio::select! {
        biased;
        Ok(_) = run.wait_for(|state| *state == MarketBulkRunState::Stopping) => true,
        () = tokio::time::sleep(pause) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::ready;

    use tokio::time::Instant;

    use super::*;

    #[test]
    fn run_claim() {
        let run = watch::Sender::new(MarketBulkRunState::Idle);
        stop(&run);
        assert_eq!(*run.borrow(), MarketBulkRunState::Idle);

        assert!(claim(&run));
        assert!(!claim(&run));
        stop(&run);
        assert!(!claim(&run));
        assert_eq!(*run.borrow(), MarketBulkRunState::Stopping);
    }

    #[tokio::test(start_paused = true)]
    async fn bulk_stop() {
        let attempts = RefCell::new(Vec::new());
        let run = async |state, pause, answer: fn() -> wf_market::Result<()>| {
            let (sender, receiver) = watch::channel(state);
            let stopper = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                stop(&sender);
                sender
            });
            let started = Instant::now();
            let outcome = paced_writes(
                vec![1, 2, 3],
                pause,
                receiver,
                |_| {},
                |target| {
                    attempts.borrow_mut().push(target);
                    ready(answer())
                },
                |_, _| {},
            )
            .await
            .unwrap();
            let elapsed = started.elapsed();
            stopper.await.unwrap();

            (outcome, attempts.take(), elapsed)
        };
        let accepted = || Ok(());
        let rate_limited = || Err(MarketError::RateLimited(Duration::from_secs(30)));

        let (outcome, tried, _) = run(MarketBulkRunState::Stopping, Duration::ZERO, accepted).await;
        assert_eq!(outcome, MarketBulkOutcome::Stopped { written: 0 });
        assert!(tried.is_empty(), "{tried:?}");

        let (outcome, tried, elapsed) = run(
            MarketBulkRunState::Running,
            Duration::from_mins(1),
            accepted,
        )
        .await;
        assert_eq!(outcome, MarketBulkOutcome::Stopped { written: 1 });
        assert_eq!(tried, [1]);
        assert_eq!(elapsed, Duration::from_secs(5));

        let (outcome, tried, elapsed) =
            run(MarketBulkRunState::Running, Duration::ZERO, rate_limited).await;
        assert_eq!(outcome, MarketBulkOutcome::Stopped { written: 0 });
        assert_eq!(tried, [1]);
        assert_eq!(elapsed, Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn bulk_failures() {
        let (_run, receiver) = watch::channel(MarketBulkRunState::Running);
        let attempts = RefCell::new(Vec::new());
        let reported = RefCell::new(Vec::new());
        let failures = RefCell::new(Vec::new());

        let failure = paced_writes(
            vec![1, 2, 3],
            Duration::ZERO,
            receiver,
            |progress: MarketBulkProgress| {
                reported
                    .borrow_mut()
                    .push((progress.written, progress.total));
            },
            |target| {
                attempts.borrow_mut().push(target);
                let tries = attempts.borrow().len();
                ready(match (target, tries) {
                    (1, 1) => Err(MarketError::RateLimited(Duration::from_secs(1))),
                    (2, _) => Err(MarketError::Unauthorized),
                    _ => Ok(()),
                })
            },
            |target, _| failures.borrow_mut().push(target),
        )
        .await
        .unwrap_err();

        assert_eq!(failure.written, 1);
        assert!(matches!(failure.error, MarketError::Unauthorized));
        assert_eq!(attempts.into_inner(), [1, 1, 2]);
        assert_eq!(reported.into_inner(), [(1, 3)]);
        assert_eq!(failures.into_inner(), [2]);
    }
}
