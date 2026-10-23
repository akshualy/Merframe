use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Runtime};
use tracing::debug;
use wf_core::{KeptRoll, RivenRow, riven_display_name, riven_kept_roll};

use super::screens::PendingAnswer;
use super::{Kind, RivenTrigger, enabled, lua_read, show, update_slots};
use crate::runtime::{AppEvent, blocking, emit};
use crate::state::{AppState, lock, read};

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

fn station_read(state: &Arc<AppState>) -> Option<wf_scan::StationRiven> {
    lua_read(state, |reader, lua| {
        wf_scan::StationRiven::read(reader, lua, &mut lock(&state.overlays.station_script))
    })
}

fn dialog_read(state: &Arc<AppState>) -> Option<wf_scan::DialogRiven> {
    lua_read(state, |reader, lua| {
        wf_scan::DialogRiven::read(reader, lua, &mut lock(&state.overlays.dialog_script))
    })
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

fn shown_roll<T>(state: &AppState, inspect: impl FnOnce(&RivenRow) -> T) -> Option<T> {
    lock(&state.overlays.state)
        .riven
        .as_ref()
        .and_then(|trigger| trigger.before.as_ref())
        .map(inspect)
}

fn awaits_cycled_roll(state: &Arc<AppState>, row: &RivenRow) -> bool {
    shown_roll(state, |before| {
        before.weapon_path == row.weapon_path && before.pending.is_none()
    })
    .unwrap_or(false)
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
            let held = update_slots(&app, &owned, |slots| {
                let shown = slots
                    .riven
                    .as_mut()
                    .and_then(|trigger| trigger.before.as_mut());
                match shown {
                    Some(shown) if shown.weapon_path == cycled.weapon_path => {
                        *shown = cycled;
                        true
                    }
                    _ => false,
                }
            });
            if held {
                emit(&app, AppEvent::InventoryUpdated(owned.status_snapshot()));
            }

            return;
        }
    });
}

fn settle_choice<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, kept: &str) {
    let Some(shown) = shown_roll(state, RivenRow::clone) else {
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
        emit(app, AppEvent::InventoryUpdated(state.status_snapshot()));
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

async fn linked_dialog(state: &Arc<AppState>, generation: u64) -> Option<wf_scan::DialogRiven> {
    for _ in 0..DIALOG_TICKS {
        let reading = Arc::clone(state);
        let found = blocking("read the riven dialog", move || dialog_read(&reading)).await?;
        if !lock(&state.overlays.screens).chat_link_pending(generation) {
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
    generation: u64,
) {
    if !riven_scans(state) {
        return;
    }
    let app = app.clone();
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let Some(dialog) = linked_dialog(&owned, generation).await else {
            debug!(item_type, "Riven dialog holds no roll");
            return;
        };
        let (linked, attribution) = {
            let core = lock(&owned.core);
            (
                core.riven_in_dialog(&dialog.mod_type, &dialog.fingerprint),
                core.riven_attribution(),
            )
        };
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
            attribution,
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
        if !lock(&owned.overlays.screens).cycling.is_open() {
            return;
        }
        if station.offered.is_none() && lock(&owned.overlays.screens).cycling.riven_resolved() {
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
            if !lock(&owned.overlays.screens).cycling.awaiting_riven() {
                return;
            }
            let Some(shown) =
                shown_roll(&owned, RivenRow::clone).filter(|shown| shown.pending.is_some())
            else {
                lock(&owned.overlays.screens).cycling.riven_resolved();
                return;
            };
            let Some((station, row)) = station_selection(&owned).await else {
                continue;
            };
            if station.offered.is_some() || riven_kept_roll(&shown, &station.shown).is_none() {
                continue;
            }
            lock(&owned.overlays.screens).cycling.riven_resolved();
            settle_choice(&app, &owned, &station.shown);
            show_riven(&app, &owned, row);
            return;
        }
    });
}

pub(super) fn arm_station_answer(state: &Arc<AppState>, at: f64) {
    if !riven_scans(state) {
        return;
    }
    let owned = Arc::clone(state);
    tauri::async_runtime::spawn(async move {
        let answer = match station_selection(&owned).await {
            Some((station, _)) if station.offered.is_some() => PendingAnswer::KeepRoll,
            Some((_, row)) => PendingAnswer::Cycle { row: Box::new(row) },
            None => return,
        };
        lock(&owned.overlays.screens).cycling.arm(answer, at);
    });
}

pub(super) fn on_dialog_answer<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, at: f64) {
    let answer = lock(&state.overlays.screens).cycling.confirmed_answer(at);
    match answer {
        Some(PendingAnswer::Cycle { row }) => {
            debug!(
                riven = riven_display_name(&row),
                rerolls = row.rerolls,
                "Cycle confirmed"
            );
            lock(&state.overlays.screens).cycling.riven_resolved();
            show_riven(app, state, (*row).clone());
            hold_cycled_roll(app, state, &row);
        }
        Some(PendingAnswer::KeepRoll) => {
            debug!("Roll chosen, reading station card");
            lock(&state.overlays.screens).cycling.roll_chosen();
            settle_after_choice(app, state);
        }
        None => {}
    }
}

pub(super) fn show_riven<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>, row: RivenRow) {
    let trigger = RivenTrigger {
        item_type: row.item_type.clone(),
        before: Some(row),
        linked: None,
        attribution: lock(&state.core).riven_attribution(),
    };
    show(app, state, Kind::Riven, |slots| {
        slots.riven = Some(trigger);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
