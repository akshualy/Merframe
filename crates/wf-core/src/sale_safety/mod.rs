use std::cmp::Ordering;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wf_data::Item;

use crate::catalog::own_blueprint;
use crate::identity::{ItemKind, ItemRecord, ItemTable, held_names};
use crate::inventory_view::{complete_sets, held_parts, is_part_stock};
use crate::listings::MarketListings;
use crate::view::View;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SaleSafetyKeep {
    #[default]
    Unmastered,
    Unbuilt,
    Nothing,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SaleSafetyRule {
    pub keep: SaleSafetyKeep,
    pub keep_favourites: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SaleSafetyReason {
    Mastery,
    Ownership,
    Favourite,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SaleSafetyReservation {
    pub item: String,
    pub via: Option<String>,
    pub reason: SaleSafetyReason,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SaleSafetyState {
    #[default]
    NothingToList,
    MoreToList {
        more: i64,
    },
    AllListed,
    OverListed {
        over: i64,
    },
}

impl SaleSafetyState {
    fn of(spare: i64, listed: i64) -> Self {
        match listed.cmp(&spare) {
            Ordering::Greater => Self::OverListed {
                over: listed - spare,
            },
            Ordering::Equal if spare == 0 => Self::NothingToList,
            Ordering::Equal => Self::AllListed,
            Ordering::Less => Self::MoreToList {
                more: spare - listed,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct SaleSafety {
    pub reserved: i64,
    pub reserved_for: Vec<SaleSafetyReservation>,
    pub listed: Option<i64>,
    pub listed_as_parts: i64,
    pub state: SaleSafetyState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct MarketItemSaleSafety {
    pub owned: i64,
    pub order_limit: i64,
    pub safety: SaleSafety,
}

#[derive(Clone, Copy)]
enum SaleSafetyWant {
    Keep(SaleSafetyReason),
    Craft,
}

impl SaleSafetyWant {
    fn reason(self) -> SaleSafetyReason {
        match self {
            Self::Keep(reason) => reason,
            Self::Craft => SaleSafetyReason::Mastery,
        }
    }
}

struct SaleSafetyListedCopies<'a> {
    direct: HashMap<&'a str, i64>,
    sets: HashMap<&'a str, i64>,
}

impl<'a> SaleSafetyListedCopies<'a> {
    fn new(listings: &MarketListings, items: &'a ItemTable) -> Option<Self> {
        let mut listed = Self {
            direct: HashMap::new(),
            sets: HashMap::new(),
        };
        for (item_id, &quantity) in listings.selling()? {
            let Some(record) = items.by_market_id(item_id) else {
                continue;
            };
            let copies = match record.kind {
                ItemKind::Part { .. } => &mut listed.direct,
                ItemKind::Set { .. } => &mut listed.sets,
                _ => continue,
            };
            *copies.entry(record.unique_name.as_str()).or_default() += i64::from(quantity);
        }
        Some(listed)
    }

    fn direct(&self, unique_name: &str) -> i64 {
        self.direct.get(unique_name).copied().unwrap_or_default()
    }

    fn sets(&self, unique_name: &str) -> i64 {
        self.sets.get(unique_name).copied().unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
struct SaleSafetyPartListing {
    direct: i64,
    in_sets: i64,
}

struct SaleSafetyPartTally<'s> {
    owned: i64,
    reserved: i64,
    listing: Option<SaleSafetyPartListing>,
    users: &'s [(&'s Item, i64)],
}

impl SaleSafetyPartTally<'_> {
    fn spare(&self) -> i64 {
        self.owned - self.reserved
    }

    fn listed(&self) -> Option<i64> {
        self.listing.map(|listing| listing.direct + listing.in_sets)
    }

    fn order_limit(&self) -> i64 {
        let in_sets = self.listing.map_or(0, |listing| listing.in_sets);
        (self.spare() - in_sets).max(0)
    }
}

fn counts_as_part(record: &ItemRecord) -> bool {
    record.tradable()
        && (is_part_stock(&record.unique_name)
            || held_names(&record.unique_name).any(|held| is_part_stock(&held)))
}

fn fewest_sets<'s>(
    parts: &[(SaleSafetyPartTally<'s>, i64)],
    copies: impl Fn(&SaleSafetyPartTally<'s>) -> i64,
) -> i64 {
    complete_sets(parts.iter().map(|(tally, needed)| (copies(tally), *needed)))
}

fn recipes<'a>(view: &View<'a>) -> impl Iterator<Item = &'a Item> {
    view.catalog
        .items()
        .filter(|item| item.components.is_some())
}

struct SaleSafetyRecipes<'a> {
    view: View<'a>,
    rule: SaleSafetyRule,
    used_in: HashMap<&'a str, Vec<(&'a Item, i64)>>,
}

impl<'a> SaleSafetyRecipes<'a> {
    fn new(view: &View<'a>, rule: SaleSafetyRule) -> Self {
        let mut used_in: HashMap<&'a str, Vec<(&'a Item, i64)>> = HashMap::new();
        for item in recipes(view) {
            for component in item.components.iter().flatten() {
                let users = used_in.entry(component.unique_name.as_str()).or_default();
                let count = i64::from(component.item_count);
                match users.last_mut() {
                    Some((recipe, needed)) if std::ptr::eq(*recipe, item) => *needed += count,
                    _ => users.push((item, count)),
                }
            }
        }

        Self {
            view: *view,
            rule,
            used_in,
        }
    }

    fn users(&self, unique_name: &str) -> &[(&'a Item, i64)] {
        self.used_in.get(unique_name).map_or(&[], Vec::as_slice)
    }

    fn copies(&self, item: &Item) -> i64 {
        let account = self.view.account;
        let unique_name = item.unique_name.as_str();
        let held = account
            .foundry
            .weapon_copies(unique_name)
            .unwrap_or_else(|| i64::from(account.built(unique_name) || account.owns(unique_name)));
        held + account.building(item)
    }

    fn want(&self, item: &Item) -> Option<SaleSafetyWant> {
        if self.rule.keep_favourites && self.view.favourites.contains(&item.unique_name) {
            return Some(SaleSafetyWant::Keep(SaleSafetyReason::Favourite));
        }

        match self.rule.keep {
            SaleSafetyKeep::Unbuilt => Some(SaleSafetyWant::Keep(SaleSafetyReason::Ownership)),
            SaleSafetyKeep::Unmastered
                if self.copies(item) == 0
                    && item.masterable()
                    && !self.view.account.mastered(item) =>
            {
                Some(SaleSafetyWant::Craft)
            }
            SaleSafetyKeep::Unmastered | SaleSafetyKeep::Nothing => None,
        }
    }

    fn craft_counts(&self) -> HashMap<&'a str, i64> {
        let mut crafts = HashMap::new();
        for item in recipes(&self.view) {
            self.count_crafts(item, &mut crafts);
        }
        crafts
    }

    fn count_crafts(&self, item: &'a Item, crafts: &mut HashMap<&'a str, i64>) -> i64 {
        if let Some(&known) = crafts.get(item.unique_name.as_str()) {
            return known;
        }

        let consumed: i64 = self
            .users(&item.unique_name)
            .iter()
            .map(|&(user, count)| count * self.count_crafts(user, crafts))
            .sum();
        let copies = self.copies(item);
        let needed = match self.want(item) {
            Some(SaleSafetyWant::Keep(_)) => (consumed + 1 - copies).max(0),
            Some(SaleSafetyWant::Craft) => consumed.max(1),
            None => (consumed - copies).max(0),
        };
        crafts.insert(item.unique_name.as_str(), needed);
        needed
    }
}

pub struct SaleSafetyCheck<'a> {
    recipes: SaleSafetyRecipes<'a>,
    crafts: HashMap<&'a str, i64>,
    listed: Option<SaleSafetyListedCopies<'a>>,
}

impl<'a> SaleSafetyCheck<'a> {
    pub(crate) fn new(view: &View<'a>, rule: SaleSafetyRule) -> Self {
        let recipes = SaleSafetyRecipes::new(view, rule);
        Self {
            crafts: recipes.craft_counts(),
            recipes,
            listed: SaleSafetyListedCopies::new(view.listings, view.items),
        }
    }

    fn crafts(&self, item: &Item) -> i64 {
        self.crafts
            .get(item.unique_name.as_str())
            .copied()
            .unwrap_or_default()
    }

    fn reservations<'s>(
        &'s self,
        users: impl IntoIterator<Item = &'s (&'a Item, i64)>,
    ) -> Vec<SaleSafetyReservation> {
        let mut found = Vec::new();
        for &(user, _) in users {
            self.collect_reservations(user, None, &mut found);
        }
        found.sort();
        found.dedup();
        found
    }

    fn collect_reservations(
        &self,
        item: &'a Item,
        via: Option<&'a str>,
        found: &mut Vec<SaleSafetyReservation>,
    ) {
        if self.crafts(item) == 0 {
            return;
        }

        if let Some(want) = self.recipes.want(item)
            && self.recipes.copies(item) == 0
        {
            found.push(SaleSafetyReservation {
                item: item.name.clone(),
                via: via.map(str::to_owned),
                reason: want.reason(),
            });
        }

        let via = via.or(Some(item.name.as_str()));
        for &(consumer, _) in self.recipes.users(&item.unique_name) {
            self.collect_reservations(consumer, via, found);
        }
    }

    fn part_tally(&self, record: &ItemRecord) -> SaleSafetyPartTally<'_> {
        let account = self.recipes.view.account;
        let unique_name = record.unique_name.as_str();
        let users = self.recipes.users(unique_name);
        let demand: i64 = users
            .iter()
            .map(|&(user, count)| count * self.crafts(user))
            .sum();
        let built_form = if is_part_stock(unique_name) {
            0
        } else {
            account.stock.exact(unique_name)
        };
        let builds_a_user = users
            .iter()
            .any(|&(user, _)| own_blueprint(user) == Some(unique_name));
        let building = if builds_a_user {
            0
        } else {
            account.pending_builds(unique_name)
        };
        let covered = built_form + building;
        let owned = account.part_copies(unique_name);
        SaleSafetyPartTally {
            owned,
            reserved: owned.min((demand - covered).max(0)),
            listing: self.listed.as_ref().map(|listed| SaleSafetyPartListing {
                direct: listed.direct(unique_name),
                in_sets: users
                    .iter()
                    .map(|&(user, count)| count * listed.sets(&user.unique_name))
                    .sum(),
            }),
            users,
        }
    }

    fn safety<'s>(
        &'s self,
        users: impl IntoIterator<Item = &'s (&'a Item, i64)>,
        reserved: i64,
        spare: i64,
        listed: Option<i64>,
        listed_as_parts: i64,
    ) -> SaleSafety {
        SaleSafety {
            reserved,
            reserved_for: if reserved > 0 {
                self.reservations(users)
            } else {
                Vec::new()
            },
            listed,
            listed_as_parts,
            state: SaleSafetyState::of(spare, listed.unwrap_or_default()),
        }
    }

    fn set_safety(&self, item: &Item) -> Option<MarketItemSaleSafety> {
        let items = self.recipes.view.items;
        let parts: Vec<(SaleSafetyPartTally, i64)> =
            held_parts(items, item, self.recipes.view.account)
                .iter()
                .map(|part| (self.part_tally(part.record), part.required))
                .collect();
        if parts.is_empty() {
            return None;
        }

        let owned = fewest_sets(&parts, |tally| tally.owned);
        let spare = fewest_sets(&parts, SaleSafetyPartTally::spare);
        let order_limit = fewest_sets(&parts, |tally| {
            tally.spare() - tally.listing.map_or(0, |listing| listing.direct)
        });
        let set_orders = self
            .listed
            .as_ref()
            .map(|listed| listed.sets(&item.unique_name));
        if owned == 0 && set_orders.unwrap_or_default() == 0 {
            return None;
        }

        let reserved = owned - spare;
        let listed_as_parts = spare - order_limit;
        let listed = set_orders.map(|orders| orders + listed_as_parts);
        Some(MarketItemSaleSafety {
            owned,
            order_limit,
            safety: self.safety(
                parts.iter().flat_map(|(tally, _)| tally.users),
                reserved,
                spare,
                listed,
                listed_as_parts,
            ),
        })
    }

    pub fn market_item(&self, record: &ItemRecord) -> Option<MarketItemSaleSafety> {
        match record.kind {
            ItemKind::Part { .. } if counts_as_part(record) => {
                let tally = self.part_tally(record);
                Some(MarketItemSaleSafety {
                    owned: tally.owned,
                    order_limit: tally.order_limit(),
                    safety: self.safety(
                        tally.users,
                        tally.reserved,
                        tally.spare(),
                        tally.listed(),
                        0,
                    ),
                })
            }
            ItemKind::Set { .. } => {
                self.set_safety(self.recipes.view.catalog.item(&record.unique_name)?)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
