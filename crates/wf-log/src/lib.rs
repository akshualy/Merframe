mod dbgwin;
mod error;
mod event;
mod line;
mod paths;
mod run;
mod source;
mod tailer;
#[cfg(not(windows))]
mod tap;

pub use dbgwin::{DebugMessage, FRAME_SIZE, line_from_frame, parse_frame};
pub use error::{LogError, Result};
pub use event::{DialogButtons, Event, MonitorRect, classify};
pub use line::{Channel, Level, LogLine, parse_line};
pub use paths::default_log_path;
pub use run::is_arbitration_drone;
pub use source::{Dedup, Origin, Selection, SourceLine, TAP_INTERVAL, lines};
pub use tailer::{read_all, tail};
#[cfg(not(windows))]
pub use tap::{BufferCursor, LogTap};

#[cfg(windows)]
pub use dbgwin::DbgWinListener;

#[cfg(test)]
mod ee_log_fixture_tests {
    use crate::{
        DialogButtons, Event, MonitorRect, classify, is_arbitration_drone, parse_line, read_all,
    };

    pub(crate) fn fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/EE.log"
        ))
    }

    fn arbitration_fixture_events() -> Vec<Event> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/arbitration_survival.log"
        );
        read_all(std::path::Path::new(path))
            .unwrap()
            .iter()
            .filter_map(classify)
            .collect()
    }

    #[test]
    fn arbitration_fixture_run_lines() {
        let events = arbitration_fixture_events();
        let count = |predicate: &dyn Fn(&Event) -> bool| {
            events.iter().filter(|event| predicate(event)).count()
        };
        assert_eq!(
            count(&|event| matches!(event, Event::ClockAnchor { .. })),
            1
        );
        assert_eq!(
            count(&|event| matches!(event, Event::MissionPending { elite: true, .. })),
            2
        );
        assert_eq!(
            count(&|event| matches!(
                event,
                Event::MissionLoading {
                    host: true,
                    elite: true,
                    ..
                }
            )),
            1
        );
        assert_eq!(
            count(&|event| matches!(event, Event::SessionStarted { host: true })),
            1
        );
        assert_eq!(
            count(
                &|event| matches!(event, Event::MissionStateStarted { mission_type } if mission_type == "MT_SURVIVAL")
            ),
            1
        );
        assert_eq!(count(&|event| matches!(event, Event::SurvivalStarted)), 1);
        assert_eq!(
            count(&|event| matches!(event, Event::SurvivalReward { .. })),
            1
        );
        let agents = count(&|event| matches!(event, Event::AgentCreated { .. }));
        let drones = count(
            &|event| matches!(event, Event::AgentCreated { agent, .. } if is_arbitration_drone(agent)),
        );
        assert_eq!(agents, 451);
        assert_eq!(drones, 7);
        assert_eq!(count(&|event| matches!(event, Event::PlayersExtracting)), 1);
        assert_eq!(
            count(
                &|event| matches!(event, Event::MissionScore { node_id } if node_id == "ClanNode11")
            ),
            1
        );
        assert_eq!(
            count(&|event| matches!(event, Event::MissionLocationUnlocked)),
            1
        );
        assert_eq!(
            count(
                &|event| matches!(event, Event::LevelLoaded { path } if path.starts_with("/Lotus/Levels/Proc/PlayerShip/"))
            ),
            1
        );
    }

    fn fixture_events() -> Vec<Event> {
        read_all(&fixture_path())
            .unwrap()
            .iter()
            .filter_map(classify)
            .collect()
    }

    #[test]
    fn fixture_game_monitor() {
        let lines = read_all(&fixture_path()).unwrap();
        let monitors: Vec<MonitorRect> = lines
            .iter()
            .filter_map(|line| match classify(line) {
                Some(Event::GameMonitor(rect)) => Some(rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            monitors,
            [MonitorRect {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            }]
        );
    }

    #[test]
    fn fixture_trade_block() {
        let lines = read_all(&fixture_path()).unwrap();
        let events: Vec<Event> = lines
            .iter()
            .filter(|line| (12_347.0..12_353.0).contains(&line.time))
            .filter_map(classify)
            .collect();
        assert_eq!(
            events,
            [
                Event::ConfirmDialog {
                    buttons: DialogButtons::OkCancel,
                },
                Event::DialogAnswered { accepted: true },
                Event::TradeScreen { visible: false },
            ]
        );
    }

    #[test]
    fn fixture_dialogs() {
        let events = fixture_events();
        let count = |wanted: &Event| events.iter().filter(|event| *event == wanted).count();
        assert_eq!(
            count(&Event::ConfirmDialog {
                buttons: DialogButtons::YesNo,
            }),
            4
        );
        assert_eq!(count(&Event::DialogAnswered { accepted: true }), 15);
    }

    #[test]
    fn fixture_relic_missions() {
        let events = fixture_events();

        let count = |predicate: &dyn Fn(&Event) -> bool| {
            events.iter().filter(|event| predicate(event)).count()
        };

        assert_eq!(
            count(&|event| matches!(event, Event::RelicRewardScreenOpened)),
            3
        );
        assert_eq!(count(&|event| matches!(event, Event::MissionSucceeded)), 19);
        assert!(count(&|event| matches!(event, Event::SquadRewardInfoReceived { .. })) >= 4);

        let own_relic_rewards: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                Event::OwnRelicReward { store_item, .. } => Some(store_item.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            own_relic_rewards
                .iter()
                .any(|item| item.ends_with("PrimeDaikyuBlueprint"))
        );
        assert!(
            own_relic_rewards
                .iter()
                .any(|item| item.ends_with("SevagothPrimeSystemsBlueprint"))
        );

        assert!(events.contains(&Event::LoggedIn {
            name: "TestTenno".to_owned()
        }));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::MissionSet { void_tier: Some(tier), .. } if tier == "VoidT1"
        )));
    }

    #[test]
    fn fixture_overlay_hide_triggers() {
        let events = fixture_events();

        let resets = events
            .iter()
            .filter(|event| matches!(event, Event::InputMappingReset))
            .count();
        assert_eq!(resets, 40);

        let consoles = events
            .iter()
            .filter(|event| matches!(event, Event::ConsoleOpened))
            .count();
        assert_eq!(consoles, 22);

        let counters: Vec<u32> = events
            .iter()
            .filter_map(|event| match event {
                Event::PurchaseDialogHudVisibility { count } => Some(*count),
                _ => None,
            })
            .collect();
        assert_eq!(counters, vec![2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1]);
    }

    #[test]
    fn fixture_star_chart_marks() {
        let star_chart = fixture_events()
            .iter()
            .filter(|event| matches!(event, Event::StarChartOpened))
            .count();
        assert_eq!(star_chart, 40);
    }

    #[test]
    fn fixture_window_focus() {
        let lines = read_all(&fixture_path()).unwrap();
        let focus: Vec<bool> = lines
            .iter()
            .filter_map(|line| match classify(line) {
                Some(Event::WindowFocus { focused }) => Some(focused),
                _ => None,
            })
            .collect();

        let text = std::fs::read_to_string(fixture_path()).unwrap();
        let written = text.matches("WM_ACTIVATEAPP ").count();
        let unparsable = text
            .lines()
            .filter(|line| line.contains("WM_ACTIVATEAPP") && parse_line(line).is_none())
            .count();
        assert_eq!(written, 164);
        assert_eq!(unparsable, 2);

        assert_eq!(focus.len(), written - unparsable);
        assert_eq!(focus.iter().filter(|f| **f).count(), 81);
        assert_eq!(focus.iter().filter(|f| !**f).count(), 81);
        assert_eq!(focus.first(), Some(&true));
    }

    #[test]
    fn fixture_inventory_sync() {
        let lines = read_all(&fixture_path()).unwrap();
        let text = std::fs::read_to_string(fixture_path()).unwrap();
        let synced = lines
            .iter()
            .filter(|line| classify(line) == Some(Event::InventorySynced))
            .count();
        let committed = lines
            .iter()
            .filter(|line| classify(line) == Some(Event::InventoryCommitted))
            .count();
        let expected = text.matches("SyncInventoryFromDB").count()
            + text.matches("OnInventoryResults completed in").count()
            + text.matches("Inventory sync done").count();
        assert_eq!(expected, 59);
        assert_eq!(synced, expected);
        assert_eq!(
            committed,
            text.matches("CommitInventoryChangesToDB").count()
        );
        assert_eq!(committed, 12);
    }
}
