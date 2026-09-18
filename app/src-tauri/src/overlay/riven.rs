use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};
use wf_core::{KeptRoll, RivenRow, riven_display_name, riven_kept_roll};

use super::{
    Kind, Marks, PendingAnswer, RivenTrigger, attached_game, broadcast, enabled, show, since,
};
use crate::runtime::{INVENTORY_UPDATED, blocking, emit};
use crate::state::{AppState, lock, read};

const PURCHASE_DIALOG_WINDOW: f64 = 2.0;
const RIVEN_REPEAT: f64 = 0.4;
const DIALOG_ANSWER_WINDOW: f64 = 300.0;
const PENDING_TICK: Duration = Duration::from_millis(100);
const PENDING_TICKS: usize = 400;
const DIALOG_TICK: Duration = Duration::from_millis(100);
const DIALOG_TICKS: usize = 20;

pub fn riven_item_type(item_path: &str) -> Option<String> {
    if !item_path.contains("/Upgrades/Mods/Randomized/") {
        return None;
    }
    Some(item_path.replacen("/Lotus/StoreItems/", "/Lotus/", 1))
}

fn confirmed_answer(marks: &mut Marks, at: f64) -> Option<PendingAnswer> {
    let (answer, armed) = marks.pending.take()?;
    (at - armed < DIALOG_ANSWER_WINDOW).then_some(answer)
}

pub(super) fn station_closes(marks: &mut Marks, at: f64) -> bool {
    if marks.reroll_screen.is_some_and(|opened| at > opened) {
        marks.reroll_screen = None;
        return true;
    }
    false
}

pub(super) fn riven_shows(marks: &Marks, at: f64) -> bool {
    since(marks.purchase_dialog, at)
        .is_some_and(|elapsed| (0.0..PURCHASE_DIALOG_WINDOW).contains(&elapsed))
        && !since(marks.riven, at).is_some_and(|elapsed| elapsed < RIVEN_REPEAT)
}

pub(super) fn purchase_dialog_closes(marks: &mut Marks, count: u32, at: f64) -> bool {
    let dropped = count < marks.hud_visibility;
    marks.hud_visibility = count;
    marks.purchase_dialog = (!dropped).then_some(at);
    dropped
}

fn lua_read<T>(
    state: &Arc<AppState>,
    read: impl FnOnce(&(dyn wf_mem::MemoryReader + 'static), &wf_scan::LuaState) -> Option<T>,
) -> Option<T> {
    let reader = attached_game()?;
    let mut located = lock(&state.overlays.lua);
    if !located.is_some_and(|lua| lua.holds(reader.as_ref())) {
        *located = match wf_scan::LuaState::locate(reader.as_ref()) {
            Ok(found) => found,
            Err(error) => {
                warn!(%error, "Lua state read from game memory failed");
                None
            }
        };
    }
    read(reader.as_ref(), located.as_ref()?)
}

fn station_read(state: &Arc<AppState>) -> Option<wf_scan::StationRiven> {
    lua_read(state, wf_scan::StationRiven::read)
}

fn dialog_read(state: &Arc<AppState>) -> Option<wf_scan::DialogRiven> {
    lua_read(state, wf_scan::DialogRiven::read)
}

fn cycled_roll(state: &Arc<AppState>, row: &RivenRow) -> Option<RivenRow> {
    let station = station_read(state)?;
    if station.item_id != row.item_id {
        return None;
    }
    let offered = station.offered.as_deref()?;
    let mut core = lock(&state.core);
    let cycled = core.riven_cycled(row, &station.shown, offered)?;
    if cycled.rerolls <= row.rerolls {
        return None;
    }
    core.apply_reroll(&station.item_id, &station.shown, offered);
    Some(cycled)
}

fn awaits_cycled_roll(state: &Arc<AppState>, row: &RivenRow) -> bool {
    lock(&state.overlays.state)
        .riven
        .as_ref()
        .and_then(|trigger| trigger.before.as_ref())
        .is_some_and(|before| before.weapon_path == row.weapon_path && before.pending.is_none())
}

fn hold_cycled_roll<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, row: &RivenRow) {
    let app = app.clone();
    let owned = Arc::clone(state);
    let row = row.clone();
    tauri::async_runtime::spawn(async move {
        for tick in 1..=PENDING_TICKS {
            tokio::time::sleep(PENDING_TICK).await;
            if !awaits_cycled_roll(&owned, &row) {
                return;
            }
            let scanning = Arc::clone(&owned);
            let wanted = row.clone();
            let found = blocking("read the cycled roll", move || {
                cycled_roll(&scanning, &wanted)
            })
            .await;
            let cycled = match found {
                Some(Some(cycled)) => cycled,
                Some(None) => continue,
                None => return,
            };
            debug!(
                riven = cycled.weapon,
                roll = cycled.name,
                rerolls = cycled.rerolls,
                tick,
                "Reroll station shows a new roll"
            );
            let mut slots = lock(&owned.overlays.state);
            let shown = slots
                .riven
                .as_mut()
                .and_then(|trigger| trigger.before.as_mut());
            let held = match shown {
                Some(shown) if shown.weapon_path == cycled.weapon_path => {
                    *shown = cycled;
                    true
                }
                _ => false,
            };
            if held {
                slots.seq += 1;
            }
            drop(slots);
            if held {
                broadcast(&app, &owned);
                emit(&app, INVENTORY_UPDATED, owned.status_snapshot());
            }
            return;
        }
    });
}

fn settle_choice<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kept: &str) {
    let shown = lock(&state.overlays.state)
        .riven
        .as_ref()
        .and_then(|trigger| trigger.before.clone());
    let Some(shown) = shown else {
        return;
    };
    let riven = riven_display_name(&shown);
    let applied = match riven_kept_roll(&shown, kept) {
        Some(KeptRoll::Offered) => {
            debug!(riven, "Offered roll kept");
            lock(&state.core).keep_pending_roll(&shown.item_id)
        }
        Some(KeptRoll::Current) => {
            debug!(riven, "Current roll kept");
            lock(&state.core).discard_pending_roll(&shown.item_id)
        }
        None => {
            debug!(riven, "Station shows neither roll of the cycle");
            false
        }
    };
    if applied {
        emit(app, INVENTORY_UPDATED, state.status_snapshot());
    }
}

async fn station_selection(state: &Arc<AppState>) -> Option<(wf_scan::StationRiven, RivenRow)> {
    let reading = Arc::clone(state);
    let station = blocking("read reroll station", move || station_read(&reading)).await??;
    let row = lock(&state.core).riven_at_station(
        &station.item_id,
        &station.shown,
        station.offered.as_deref(),
    );
    if let Some(shown) = &row {
        debug!(
            riven = riven_display_name(shown),
            rerolls = shown.rerolls,
            offered = station.offered.is_some(),
            "Riven selected at reroll station"
        );
    } else {
        debug!(
            roll = station.shown,
            "Riven at the station grades into no roll"
        );
    }
    Some((station, row?))
}

fn riven_scans(state: &Arc<AppState>) -> bool {
    enabled(&read(&state.settings), Kind::Riven) && read(&state.status).game_detected
}

async fn linked_dialog(state: &Arc<AppState>, ticket: u64) -> Option<wf_scan::DialogRiven> {
    for _ in 0..DIALOG_TICKS {
        let reading = Arc::clone(state);
        let found = blocking("read the riven dialog", move || dialog_read(&reading)).await?;
        if state.overlays.riven_ticket.load(Ordering::Relaxed) != ticket {
            return None;
        }
        if found.is_some() {
            return found;
        }
        tokio::time::sleep(DIALOG_TICK).await;
    }
    None
}

pub(super) fn show_linked_riven<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    item_type: String,
    ticket: u64,
) {
    if !riven_scans(state) {
        return;
    }
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let Some(dialog) = linked_dialog(&owned, ticket).await else {
            debug!(item_type, "Riven dialog holds no roll");
            return;
        };
        let linked = lock(&owned.core).riven_in_dialog(&dialog.mod_type, &dialog.fingerprint);
        if let Some(row) = &linked {
            debug!(
                riven = riven_display_name(row),
                rerolls = row.rerolls,
                "Riven dialog resolved to a roll"
            );
        } else {
            debug!(
                mod_type = dialog.mod_type,
                roll = dialog.fingerprint,
                "Riven in the dialog grades into no roll"
            );
        }
        let trigger = RivenTrigger {
            item_type,
            before: None,
            linked,
        };
        show(&app, &owned, Kind::Riven, |slots| {
            slots.riven = Some(trigger);
        });
    });
}

pub(super) fn show_station_selection<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    if !riven_scans(state) {
        return;
    }
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let Some((station, row)) = station_selection(&owned).await else {
            return;
        };
        if lock(&owned.overlays.marks).reroll_screen.is_none() {
            return;
        }
        if station.offered.is_none() && std::mem::take(&mut lock(&owned.overlays.marks).choice_made)
        {
            settle_choice(&app, &owned, &station.shown);
        }
        show_riven(&app, &owned, row);
    });
}

fn settle_after_choice<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !lock(&owned.overlays.marks).choice_made {
                return;
            }
            let shown = lock(&owned.overlays.state)
                .riven
                .as_ref()
                .and_then(|trigger| trigger.before.clone())
                .filter(|shown| shown.pending.is_some());
            let Some(shown) = shown else {
                lock(&owned.overlays.marks).choice_made = false;
                return;
            };
            let Some((station, row)) = station_selection(&owned).await else {
                continue;
            };
            if station.offered.is_some() || riven_kept_roll(&shown, &station.shown).is_none() {
                continue;
            }
            lock(&owned.overlays.marks).choice_made = false;
            settle_choice(&app, &owned, &station.shown);
            show_riven(&app, &owned, row);
            return;
        }
    });
}

fn cycle_from_station<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let Some((_, row)) = station_selection(&owned).await else {
            return;
        };
        show_riven(&app, &owned, row.clone());
        hold_cycled_roll(&app, &owned, &row);
    });
}

fn kept_roll(row: &RivenRow) -> RivenRow {
    let Some(pending) = row.pending.clone() else {
        return row.clone();
    };
    RivenRow {
        name: pending.name,
        rerolls: pending.rerolls,
        polarity: pending.polarity,
        grade: pending.grade,
        attributes: pending.attributes,
        good_roll: pending.good_roll,
        pending: None,
        ..row.clone()
    }
}

fn roll_named(shown: &RivenRow, wanted: &str) -> Option<RivenRow> {
    let current = RivenRow {
        pending: None,
        ..shown.clone()
    };
    [kept_roll(shown), current].into_iter().find(|candidate| {
        riven_display_name(candidate).is_some_and(|name| name.eq_ignore_ascii_case(wanted.trim()))
    })
}

fn panel_roll(state: &Arc<AppState>, wanted: &str) -> Option<RivenRow> {
    let shown = lock(&state.overlays.state)
        .riven
        .as_ref()
        .and_then(|trigger| trigger.before.clone())?;
    roll_named(&shown, wanted)
}

pub(super) fn on_dialog_answer<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, at: f64) {
    let answer = confirmed_answer(&mut lock(&state.overlays.marks), at);
    match answer {
        Some(PendingAnswer::Cycle { riven }) => {
            if let Some(row) = panel_roll(state, &riven) {
                debug!(riven, rerolls = row.rerolls, "Cycle confirmed");
                lock(&state.overlays.marks).choice_made = false;
                show_riven(app, state, row.clone());
                hold_cycled_roll(app, state, &row);
            } else {
                debug!(riven, "Cycle confirmed for a riven the panel does not hold");
                cycle_from_station(app, state);
            }
        }
        Some(PendingAnswer::KeepRoll) => {
            debug!("Roll chosen, reading station card");
            lock(&state.overlays.marks).choice_made = true;
            settle_after_choice(app, state);
        }
        None => {}
    }
}

fn show_riven<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, row: RivenRow) {
    let trigger = RivenTrigger {
        item_type: row.item_type.clone(),
        before: Some(row),
        linked: None,
    };
    show(app, state, Kind::Riven, |slots| {
        slots.riven = Some(trigger);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wf_core::PendingRoll;

    #[test]
    fn riven_item_type_from_store_path() {
        assert_eq!(
            riven_item_type("/Lotus/StoreItems/Upgrades/Mods/Randomized/LotusRifleRandomModRare"),
            Some("/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare".to_owned())
        );
        assert_eq!(
            riven_item_type("/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare"),
            Some("/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare".to_owned())
        );
        assert_eq!(
            riven_item_type("/Lotus/StoreItems/Upgrades/Skins/Effects/DogDaysEphemera"),
            None
        );
        assert_eq!(riven_item_type(""), None);
    }

    fn roll(rerolls: u32, name: &str, pending: Option<PendingRoll>) -> RivenRow {
        RivenRow {
            item_id: "19afb0d351ebb3350bab634c".to_owned(),
            item_type: "/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare".to_owned(),
            riven_type: Some("Melee Riven Mod".to_owned()),
            weapon_class: Some("Melee".to_owned()),
            name: Some(name.to_owned()),
            weapon: Some("Nepheri".to_owned()),
            weapon_path: Some(
                "/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep".to_owned(),
            ),
            weapon_slug: Some("nepheri".to_owned()),
            image_name: Some("ArchonDualDaggers.png".to_owned()),
            disposition: Some(1.0),
            disposition_weapon: None,
            unveiled: true,
            rank: 8,
            rank_required: Some(11),
            rerolls,
            polarity: Some(wf_market::Polarity::Vazarin),
            grade: 0.5,
            attributes: Vec::new(),
            good_roll: None,
            listed_in_wfm: false,
            pending,
        }
    }

    #[test]
    fn kept_roll_promotes_offered() {
        let before = roll(
            100,
            "Geli-toxidra",
            Some(PendingRoll {
                name: Some("Acri-gelipha".to_owned()),
                rerolls: 101,
                polarity: Some(wf_market::Polarity::Vazarin),
                grade: 0.75,
                attributes: Vec::new(),
                good_roll: None,
            }),
        );
        let kept = kept_roll(&before);
        assert_eq!(kept.name.as_deref(), Some("Acri-gelipha"));
        assert_eq!(kept.rerolls, 101);
        assert_eq!(kept.pending, None);
        assert_eq!(kept.item_id, before.item_id);

        let alone = roll(100, "Geli-toxidra", None);
        assert_eq!(kept_roll(&alone), alone);
    }

    #[test]
    fn roll_named_either_side() {
        let shown = roll(
            101,
            "Geli-toxidra",
            Some(PendingRoll {
                name: Some("Acri-gelipha".to_owned()),
                rerolls: 101,
                polarity: Some(wf_market::Polarity::Vazarin),
                grade: 0.75,
                attributes: Vec::new(),
                good_roll: None,
            }),
        );

        let kept = roll_named(&shown, "Nepheri Acri-gelipha").expect("offered roll");
        assert_eq!(kept.name.as_deref(), Some("Acri-gelipha"));
        assert_eq!(kept.pending, None);

        let held = roll_named(&shown, "nepheri geli-toxidra").expect("held roll");
        assert_eq!(held.name.as_deref(), Some("Geli-toxidra"));
        assert_eq!(held.pending, None);

        assert_eq!(roll_named(&shown, "Soma Hexatox"), None);
    }

    #[test]
    fn confirmed_answer_expires() {
        let armed = |at| {
            let mut marks = Marks {
                pending: Some((PendingAnswer::KeepRoll, 96.931)),
                ..Marks::default()
            };
            confirmed_answer(&mut marks, at)
        };
        assert_eq!(armed(98.047), Some(PendingAnswer::KeepRoll));
        assert_eq!(armed(396.9), Some(PendingAnswer::KeepRoll));
        assert_eq!(armed(396.931), None);
        assert_eq!(armed(1000.0), None);

        let mut marks = Marks::default();
        assert_eq!(confirmed_answer(&mut marks, 98.047), None);
    }

    #[test]
    fn station_closes_after_open_only() {
        let mut marks = Marks::default();
        assert!(!station_closes(&mut marks, 51731.293));

        marks.reroll_screen = Some(51731.441);
        assert!(!station_closes(&mut marks, 51731.277));
        assert!(marks.reroll_screen.is_some());
        assert!(station_closes(&mut marks, 51790.0));
        assert!(!station_closes(&mut marks, 51791.0));
    }

    #[test]
    fn riven_shows_once_per_dialog() {
        let mut marks = Marks::default();
        assert!(!riven_shows(&marks, 4310.355));

        marks.purchase_dialog = Some(4309.896);
        assert!(riven_shows(&marks, 4310.355));
        assert!(!riven_shows(&marks, 4311.9));

        marks.riven = Some(4310.355);
        assert!(!riven_shows(&marks, 4310.5));
        assert!(riven_shows(&marks, 4310.8));
    }

    #[test]
    fn late_dialog_line() {
        let mut marks = Marks::default();

        assert!(!purchase_dialog_closes(&mut marks, 1, 38684.285));
        assert!(riven_shows(&marks, 38684.598));

        assert!(purchase_dialog_closes(&mut marks, 0, 38687.449));
        assert!(!riven_shows(&marks, 38684.598));

        assert!(!purchase_dialog_closes(&mut marks, 1, 38688.289));
        assert!(!riven_shows(&marks, 38684.598));
        assert!(riven_shows(&marks, 38688.48));
        marks.riven = Some(38688.48);

        assert!(purchase_dialog_closes(&mut marks, 0, 38690.582));
        assert!(!riven_shows(&marks, 38690.9));
    }

    #[test]
    fn dialog_over_menu() {
        let mut marks = Marks::default();
        assert!(!purchase_dialog_closes(&mut marks, 2, 26962.0));
        assert!(riven_shows(&marks, 26962.395));
        assert!(purchase_dialog_closes(&mut marks, 1, 26980.58));
        assert!(!riven_shows(&marks, 26980.7));

        assert!(!purchase_dialog_closes(&mut marks, 1, 38259.086));
        assert!(riven_shows(&marks, 38259.395));
        assert!(purchase_dialog_closes(&mut marks, 0, 38267.215));
        assert!(!riven_shows(&marks, 38267.3));
    }
}
