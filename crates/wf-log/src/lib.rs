mod dbgwin;
mod error;
mod event;
mod line;
mod paths;
mod source;
mod tailer;
#[cfg(not(windows))]
mod tap;

pub use dbgwin::{DebugMessage, FRAME_SIZE, line_from_frame, parse_frame};
pub use error::{LogError, Result};
pub use event::{Event, MonitorRect, classify};
pub use line::{Channel, Level, LogLine, parse_line};
pub use paths::default_log_path;
pub use source::{Dedup, Origin, Selection, SourceLine, TAP_INTERVAL, lines};
pub use tailer::{read_all, tail};
#[cfg(not(windows))]
pub use tap::{BufferCursor, LogTap};

#[cfg(windows)]
pub use dbgwin::DbgWinListener;

#[cfg(test)]
mod ee_log_fixture_tests {
    use crate::{Event, MonitorRect, classify, parse_line, read_all};

    fn fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/EE.log"
        ))
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
    fn fixture_relic_missions() {
        let lines = read_all(&fixture_path()).unwrap();
        let events: Vec<Event> = lines.iter().filter_map(classify).collect();

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
        assert!(events.contains(&Event::RelicEquipDialog {
            relic: "Lith K12".to_owned()
        }));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::MissionSet { void_tier: Some(tier), .. } if tier == "VoidT1"
        )));
    }

    #[test]
    fn fixture_overlay_hide_triggers() {
        let lines = read_all(&fixture_path()).unwrap();
        let events: Vec<Event> = lines.iter().filter_map(classify).collect();

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
        let lines = read_all(&fixture_path()).unwrap();
        let events: Vec<Event> = lines.iter().filter_map(classify).collect();

        let star_chart = events
            .iter()
            .filter(|event| matches!(event, Event::StarChartOpened))
            .count();
        assert_eq!(star_chart, 40);

        let hidden = events
            .iter()
            .filter(|event| matches!(event, Event::StarChartHidden))
            .count();
        assert_eq!(hidden, 16);
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
