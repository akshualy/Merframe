use std::fmt::Write as _;

use serde::Serialize;
use wf_core::{CoreEvent, FissureInfo, InventorySummary};

use crate::settings::InGameToast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Success,
    Info,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct Notice {
    pub title: String,
    pub body: String,
    pub tone: Tone,
    pub in_game: Option<InGameToast>,
}

impl Notice {
    pub fn for_event(event: &CoreEvent) -> Option<Self> {
        match event {
            CoreEvent::InventoryUpdated(summary) => Self::inventory(summary),
            CoreEvent::RelicRewardScreen { .. } => None,
            CoreEvent::TradeCompleted { trade, partner, .. } => {
                Some(Self::trade(trade.plat, partner.as_deref()))
            }
            CoreEvent::NewConversation { player, .. } => Some(Self::conversation(player)),
            CoreEvent::FissureAlert { fissure } => Some(Self::fissure(fissure)),
            CoreEvent::TimerAlert {
                name,
                next_state,
                remaining_secs,
                ..
            } => Some(Self::timer(name, next_state, *remaining_secs)),
        }
    }

    fn inventory(summary: &InventorySummary) -> Option<Self> {
        (summary.changes > 0).then(|| Self {
            title: format!("{} inventory changes", summary.changes),
            body: format!(
                "{} plat, {} endo, MR {}",
                thousands(summary.plat),
                thousands(summary.endo),
                summary.mr
            ),
            tone: Tone::Info,
            in_game: Some(InGameToast::Inventory),
        })
    }

    fn trade(plat: i64, partner: Option<&str>) -> Self {
        Self {
            title: String::from("Trade completed"),
            body: format!("{plat} plat with {}", partner.unwrap_or("an unknown Tenno")),
            tone: Tone::Success,
            in_game: Some(InGameToast::Trade),
        }
    }

    pub fn conversation(player: &str) -> Self {
        Self {
            title: String::from("New in-game conversation"),
            body: player.to_owned(),
            tone: Tone::Info,
            in_game: None,
        }
    }

    pub fn fissure(fissure: &FissureInfo) -> Self {
        let node = fissure.node_name.unwrap_or(fissure.node_id.as_str());
        let kind = if fissure.is_storm {
            "Void Storm"
        } else {
            "Fissure"
        };
        let mut body = fissure.mission_name.clone();
        if let Some((min, max)) = fissure.levels {
            let _ = write!(body, " ({min}-{max})");
        }
        if let Some(faction) = fissure.faction {
            let _ = write!(body, " - {faction}");
        }
        if fissure.steel_path {
            body.push_str(", Steel Path");
        }
        if fissure.is_storm {
            body.push_str(", Void Storm");
        }
        let _ = write!(body, ", {} left", countdown(fissure.remaining_secs));
        Self {
            title: format!("New {} {kind} - {node}", fissure.tier),
            body,
            tone: Tone::Warning,
            in_game: Some(InGameToast::Fissure),
        }
    }

    pub fn timer(name: &str, next_state: &str, remaining_secs: i64) -> Self {
        Self {
            title: format!("{name} turns {next_state}"),
            body: format!("in {}", countdown(remaining_secs)),
            tone: Tone::Warning,
            in_game: Some(InGameToast::Timer),
        }
    }

    pub fn webhook_message(&self) -> String {
        format!("{}\n{}", self.title, self.body)
    }
}

pub fn countdown(seconds: i64) -> String {
    if seconds <= 0 {
        return String::from("ready");
    }
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;
    let rest = seconds % 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {rest:02}s")
    } else {
        format!("{rest}s")
    }
}

fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        out.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn fissure() -> FissureInfo {
        FissureInfo {
            node_id: String::from("SolNode58"),
            node_name: Some("Hellas (Mars)"),
            mission_type: String::from("MT_EXTERMINATION"),
            mission_name: String::from("Extermination"),
            planet: Some("Mars"),
            tier: String::from("Lith"),
            steel_path: false,
            is_storm: false,
            faction: Some("Grineer"),
            levels: Some((15, 17)),
            expiry: DateTime::UNIX_EPOCH,
            remaining_secs: 5400,
        }
    }

    #[test]
    fn fissure_names_tier_node_mission_levels_and_faction() {
        let notice = Notice::fissure(&fissure());
        assert_eq!(notice.title, "New Lith Fissure - Hellas (Mars)");
        assert_eq!(notice.body, "Extermination (15-17) - Grineer, 1h 30m left");
        assert_eq!(notice.in_game, Some(InGameToast::Fissure));
    }

    #[test]
    fn fissure_modifiers() {
        let steel_path = FissureInfo {
            steel_path: true,
            levels: Some((115, 117)),
            ..fissure()
        };
        assert_eq!(
            Notice::fissure(&steel_path).body,
            "Extermination (115-117) - Grineer, Steel Path, 1h 30m left"
        );
        let storm = Notice::fissure(&FissureInfo {
            is_storm: true,
            levels: None,
            ..fissure()
        });
        assert_eq!(storm.title, "New Lith Void Storm - Hellas (Mars)");
        assert_eq!(
            storm.body,
            "Extermination - Grineer, Void Storm, 1h 30m left"
        );
    }

    #[test]
    fn timer_and_webhook_message() {
        let notice = Notice::timer("Cetus", "night", 300);
        assert_eq!(notice.title, "Cetus turns night");
        assert_eq!(notice.body, "in 5m 00s");
        assert_eq!(notice.webhook_message(), "Cetus turns night\nin 5m 00s");
    }

    #[test]
    fn inventory_without_changes_is_silent() {
        let summary = InventorySummary {
            last_sync_oid: String::from("6a9eeb1f000000000000c001"),
            mr: 14,
            plat: 1234,
            credits: 0,
            endo: 56789,
            ducats: 0,
            changes: 0,
        };
        assert_eq!(Notice::inventory(&summary), None);
        let changed = InventorySummary {
            changes: 3,
            ..summary
        };
        let notice = Notice::inventory(&changed).unwrap();
        assert_eq!(notice.title, "3 inventory changes");
        assert_eq!(notice.body, "1,234 plat, 56,789 endo, MR 14");
    }

    #[test]
    fn trade_without_partner() {
        assert_eq!(
            Notice::trade(120, None).body,
            "120 plat with an unknown Tenno"
        );
        assert_eq!(
            Notice::trade(120, Some("Tenno")).body,
            "120 plat with Tenno"
        );
    }

    #[test]
    fn countdown_units() {
        assert_eq!(countdown(0), "ready");
        assert_eq!(countdown(45), "45s");
        assert_eq!(countdown(25 * 60 + 5), "25m 05s");
        assert_eq!(countdown(3 * 3600 + 120), "3h 02m");
        assert_eq!(countdown(2 * 86_400 + 5 * 3600), "2d 5h");
    }

    #[test]
    fn thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(-1_234_567), "-1,234,567");
    }
}
