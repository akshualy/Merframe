use std::collections::HashMap;

use serde::Serialize;
use wf_data::{
    MAX_RANK, RivenStat as StatDefinition, RivenType, TraitMultipliers, rank_multiplier,
    roll_multiplier, roll_share, trait_multipliers,
};
use wf_inventory::{RivenFingerprint, RivenStat};
use wf_market::{Polarity, RivenAttribute};

use super::Grader;

const GRADE_BANDS: [(f64, &str); 10] = [
    (0.025, "F"),
    (0.125, "C-"),
    (0.225, "C"),
    (0.325, "C+"),
    (0.425, "B-"),
    (0.575, "B"),
    (0.675, "B+"),
    (0.775, "A-"),
    (0.875, "A"),
    (0.975, "A+"),
];

const COMPAT_FIXUPS: [(&str, &str); 2] = [
    (
        "/Lotus/Weapons/Tenno/Melee/Dagger/DarkDaggerBase",
        "/Lotus/Weapons/Tenno/Melee/Dagger/DarkDagger",
    ),
    (
        "/Lotus/Weapons/Tenno/Shotgun/QuadShotgunBase",
        "/Lotus/Weapons/Tenno/Shotgun/QuadShotgun",
    ),
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AttributeGrade {
    pub tag: String,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub unit: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub localization: Option<String>,
    pub value: i64,
    pub percentile: f64,
    pub grade: &'static str,
    pub multiplier: f64,
    pub rolled: Option<f64>,
    pub display: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub curse: bool,
}

pub(super) fn compat_path(compat: &str) -> &str {
    match COMPAT_FIXUPS.iter().find(|(from, _)| *from == compat) {
        Some((_, renamed)) => renamed,
        None => compat,
    }
}

impl Grader<'_> {
    pub(super) fn graded(
        &self,
        fingerprint: &RivenFingerprint,
        riven_type: Option<&RivenType>,
        disposition: Option<f64>,
    ) -> Vec<AttributeGrade> {
        let roll = Roll {
            riven_type,
            disposition,
            multipliers: trait_multipliers(fingerprint.buffs.len(), fingerprint.curses.len()),
            by_combo: &self.by_combo,
        };
        fingerprint
            .buffs
            .iter()
            .map(|stat| (stat, false))
            .chain(fingerprint.curses.iter().map(|stat| (stat, true)))
            .map(|(stat, curse)| roll.grade(stat, curse))
            .collect()
    }
}

struct Roll<'a> {
    riven_type: Option<&'a RivenType>,
    disposition: Option<f64>,
    multipliers: Option<TraitMultipliers>,
    by_combo: &'a HashMap<String, &'a RivenAttribute>,
}

impl Roll<'_> {
    fn grade(&self, stat: &RivenStat, curse: bool) -> AttributeGrade {
        let Roll {
            riven_type,
            disposition,
            multipliers,
            by_combo,
        } = *self;
        let modifier: Option<&StatDefinition> =
            riven_type.and_then(|found| found.stats.get(&stat.tag));
        let attribute = modifier.and_then(|found| by_combo.get(&found.prefix_suffix()).copied());
        let multiplier = roll_multiplier(stat.value);
        let rank_scale = rank_multiplier(MAX_RANK);
        let base = match (modifier, disposition, multipliers) {
            (Some(modifier), Some(disposition), Some(multipliers)) => {
                let share = if curse {
                    multipliers.bad
                } else {
                    multipliers.good
                };
                Some(modifier.base_at_rank_nine(disposition, share, curse) * rank_scale)
            }
            _ => None,
        };
        let (worst, best) = if curse { (1.1, 0.9) } else { (0.9, 1.1) };
        let rolled = base.map(|base| base * multiplier);
        let percentile = roll_share(multiplier, curse);
        AttributeGrade {
            name: attribute.and_then(|attribute| {
                attribute
                    .i18n
                    .get("en")
                    .map(|localized| localized.name.clone())
            }),
            slug: attribute.map(|attribute| attribute.slug.clone()),
            unit: attribute.and_then(|attribute| attribute.unit.clone()),
            prefix: modifier.map(|modifier| modifier.prefix.clone()),
            suffix: modifier.map(|modifier| modifier.suffix.clone()),
            localization: modifier.map(|modifier| modifier.localization.clone()),
            value: stat.value,
            percentile,
            grade: letter(percentile),
            multiplier,
            display: rolled
                .zip(modifier)
                .map(|(rolled, modifier)| display_value(rolled, modifier.multiplier_display())),
            rolled,
            min: base.map(|base| base * worst),
            max: base.map(|base| base * best),
            tag: stat.tag.clone(),
            curse,
        }
    }
}

fn letter(percentile: f64) -> &'static str {
    match GRADE_BANDS
        .iter()
        .find(|(threshold, _)| percentile < *threshold)
    {
        Some((_, band)) => band,
        None => "S",
    }
}

pub(super) fn display_value(rolled: f64, multiplier_display: bool) -> f64 {
    if multiplier_display {
        round_to(rolled + 1.0, 2)
    } else {
        round_to(rolled, 1)
    }
}

pub(super) fn round_to(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

pub(super) fn perfectness(attributes: &[AttributeGrade]) -> f64 {
    if attributes.is_empty() {
        return 0.0;
    }
    let total: f64 = attributes
        .iter()
        .map(|attribute| attribute.percentile)
        .sum();
    #[allow(clippy::cast_precision_loss, reason = "a riven has at most four stats")]
    let count = attributes.len() as f64;
    total / count
}

fn capitalised(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

pub(super) fn riven_name(riven_type: &RivenType, buffs: &[RivenStat]) -> Option<String> {
    let mut ordered: Vec<&RivenStat> = buffs.iter().collect();
    ordered.sort_by_key(|stat| std::cmp::Reverse(stat.value));
    let parts: Vec<&StatDefinition> = ordered
        .iter()
        .filter_map(|stat| riven_type.stats.get(&stat.tag))
        .collect();
    if parts.len() != ordered.len() {
        return None;
    }
    match parts.as_slice() {
        [first, second] => Some(format!(
            "{}{}",
            capitalised(&first.prefix),
            second.suffix.to_lowercase()
        )),
        [first, second, third] => Some(format!(
            "{}-{}{}",
            capitalised(&first.prefix),
            second.prefix.to_lowercase(),
            third.suffix.to_lowercase()
        )),
        _ => None,
    }
}

pub(super) fn polarity(pol: &str) -> Option<Polarity> {
    match pol {
        "AP_ATTACK" => Some(Polarity::Madurai),
        "AP_DEFENSE" => Some(Polarity::Vazarin),
        "AP_TACTIC" => Some(Polarity::Naramon),
        "AP_WARD" => Some(Polarity::Unairu),
        "AP_POWER" => Some(Polarity::Zenurik),
        "AP_PRECEPT" => Some(Polarity::Penjaga),
        "AP_UMBRA" => Some(Polarity::Umbra),
        _ => None,
    }
}

pub fn fingerprint_weapon(fingerprint: &str) -> Option<String> {
    let parsed = RivenFingerprint::parse(fingerprint).ok()?;
    Some(compat_path(parsed.compat.as_deref()?).to_owned())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the game compares rolls as 32 bit floats"
)]
fn game_float(value: i64) -> f32 {
    value as f32
}

pub(super) fn same_roll(attributes: &[AttributeGrade], fingerprint: &RivenFingerprint) -> bool {
    let rolled = fingerprint.buffs.iter().chain(&fingerprint.curses);
    attributes.len() == fingerprint.buffs.len() + fingerprint.curses.len()
        && attributes.iter().zip(rolled).all(|(attribute, stat)| {
            attribute.tag == stat.tag
                && game_float(attribute.value).to_bits() == game_float(stat.value).to_bits()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rivens::support::*;

    const DAMAGE_TAG: &str = "WeaponDamageAmountMod";

    const WORST_ROLL: i64 = 0;

    fn rolled_attribute(tag: &str, value: i64, curse: bool) -> AttributeGrade {
        let fixture = fixture();
        let riven_type = fixture
            .catalog
            .data()
            .riven_data()
            .riven_type(RIFLE_RIVEN)
            .expect("riven type");
        let roll = Roll {
            riven_type: Some(riven_type),
            disposition: Some(1.0),
            multipliers: trait_multipliers(2, 1),
            by_combo: &fixture.grader().by_combo,
        };
        roll.grade(
            &RivenStat {
                tag: tag.to_owned(),
                value,
            },
            curse,
        )
    }

    #[test]
    fn generated_names() {
        let tab = riven_tab();
        assert_eq!(
            by_weapon(&tab.unveiled, "Proboscis Cernos").name.as_deref(),
            Some("Sci-cronicron")
        );
        assert_eq!(
            by_weapon(&tab.unveiled, "Soma").name.as_deref(),
            Some("Toxidex")
        );
        assert_eq!(
            by_weapon(&tab.unveiled, "Buzlok").name.as_deref(),
            Some("Acrides")
        );
        assert_eq!(
            by_weapon(&tab.unveiled, "Zylok").name.as_deref(),
            Some("Pura-vexinak")
        );
        assert_eq!(
            by_weapon(&tab.unveiled, "Harmony").name.as_deref(),
            Some("Acri-plecium")
        );
        assert_eq!(
            by_weapon(&tab.unveiled, "Akstiletto").name.as_deref(),
            Some("Crita-concimag")
        );
        assert!(
            tab.unveiled
                .iter()
                .filter(|row| row.attributes.iter().filter(|a| !a.curse).count() == 2)
                .all(|row| !row.name.as_deref().unwrap_or_default().contains('-')),
            "two-buff names carry no dash"
        );
        assert!(
            tab.unveiled
                .iter()
                .filter(|row| row.attributes.iter().filter(|a| !a.curse).count() == 3)
                .all(|row| row.name.as_deref().unwrap_or_default().contains('-')),
            "three-buff names carry a dash"
        );
    }

    #[test]
    fn name_order_by_roll_value() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let mut buffs: Vec<&AttributeGrade> =
            cernos.attributes.iter().filter(|a| !a.curse).collect();
        assert_eq!(buffs.len(), 3);
        buffs.sort_by_key(|attribute| std::cmp::Reverse(attribute.value));
        let name = format!(
            "{}-{}{}",
            capitalised(buffs[0].prefix.as_deref().unwrap()),
            buffs[1].prefix.as_deref().unwrap().to_lowercase(),
            buffs[2].suffix.as_deref().unwrap().to_lowercase()
        );
        assert_eq!(cernos.name.as_deref(), Some(name.as_str()));

        let reordered = format!(
            "{}-{}{}",
            capitalised(buffs[2].prefix.as_deref().unwrap()),
            buffs[1].prefix.as_deref().unwrap().to_lowercase(),
            buffs[0].suffix.as_deref().unwrap().to_lowercase()
        );
        assert_ne!(cernos.name.as_deref(), Some(reordered.as_str()));
    }

    #[test]
    fn slash_roll_range() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        assert_eq!(cernos.rank, 8);
        assert!((cernos.disposition.unwrap() - 0.9).abs() < 1e-6);
        let slash = cernos
            .attributes
            .iter()
            .find(|attribute| attribute.tag == "WeaponSlashDamageMod")
            .expect("slash roll");
        let (rolled, min, max) = (
            slash.rolled.unwrap(),
            slash.min.unwrap(),
            slash.max.unwrap(),
        );
        assert!((min - 91.12).abs() < 0.01, "{min}");
        assert!((max - 111.37).abs() < 0.01, "{max}");
        assert!((rolled - 108.74).abs() < 0.01, "{rolled}");
        assert_eq!(slash.display, Some(108.7));
        assert_eq!(slash.slug.as_deref(), Some("slash_damage"));
        assert_eq!(slash.name.as_deref(), Some("Slash"));
        assert_eq!(slash.unit.as_deref(), Some("percent"));
    }

    #[test]
    fn inverted_curse_range() {
        let tab = riven_tab();
        let cernos = by_weapon(&tab.unveiled, "Proboscis Cernos");
        let reload = cernos
            .attributes
            .iter()
            .find(|attribute| attribute.curse)
            .expect("a curse");
        assert_eq!(reload.tag, "WeaponReloadSpeedMod");
        let (rolled, min, max) = (
            reload.rolled.unwrap(),
            reload.min.unwrap(),
            reload.max.unwrap(),
        );
        assert!(rolled < 0.0 && min < 0.0 && max < 0.0);
        assert!(min < max, "the worst case is the more negative one");
        assert!((min - -37.12).abs() < 0.01, "{min}");
        assert!((max - -30.37).abs() < 0.01, "{max}");
        assert!((reload.percentile - (1.0 - (reload.multiplier - 0.9) * 5.0)).abs() < 1e-12);
    }

    #[test]
    fn faction_damage_multiplier() {
        let tab = riven_tab();
        let faction = tab
            .unveiled
            .iter()
            .flat_map(|row| row.attributes.iter())
            .find(|attribute| attribute.tag.starts_with("WeaponFactionDamage") && !attribute.curse)
            .expect("faction roll");
        let rolled = faction.rolled.unwrap();
        let display = faction.display.unwrap();
        assert!(rolled.abs() < 1.0, "{rolled}");
        assert!(display > 1.0, "{display}");
        assert!((display - round_to(rolled + 1.0, 2)).abs() < f64::EPSILON);
    }

    #[test]
    fn grade_is_mean_share() {
        let tab = riven_tab();
        for row in &tab.unveiled {
            let expected = perfectness(&row.attributes);
            assert!((row.grade - expected).abs() < f64::EPSILON);
            assert!((0.0..=1.0).contains(&row.grade), "{}", row.grade);
        }
        assert!(tab.unveiled[0].grade >= tab.unveiled[tab.unveiled.len() - 1].grade);
    }

    #[test]
    fn letter_bands() {
        assert_eq!(letter(0.0), "F");
        assert_eq!(letter(0.024), "F");
        assert_eq!(letter(0.025), "C-");
        assert_eq!(letter(0.124), "C-");
        assert_eq!(letter(0.125), "C");
        assert_eq!(letter(0.224), "C");
        assert_eq!(letter(0.225), "C+");
        assert_eq!(letter(0.324), "C+");
        assert_eq!(letter(0.325), "B-");
        assert_eq!(letter(0.424), "B-");
        assert_eq!(letter(0.425), "B");
        assert_eq!(letter(0.574), "B");
        assert_eq!(letter(0.575), "B+");
        assert_eq!(letter(0.674), "B+");
        assert_eq!(letter(0.675), "A-");
        assert_eq!(letter(0.774), "A-");
        assert_eq!(letter(0.775), "A");
        assert_eq!(letter(0.874), "A");
        assert_eq!(letter(0.875), "A+");
        assert_eq!(letter(0.974), "A+");
        assert_eq!(letter(0.975), "S");
        assert_eq!(letter(1.0), "S");
    }

    #[test]
    fn worst_roll_grades_f() {
        let damage = rolled_attribute(DAMAGE_TAG, WORST_ROLL, false);
        assert!(damage.percentile.abs() < 1e-9, "{}", damage.percentile);
        assert_eq!(damage.grade, "F");
        let (rolled, min) = (damage.rolled.unwrap(), damage.min.unwrap());
        assert!((rolled - min).abs() < 1e-9, "{rolled} {min}");
    }

    #[test]
    fn best_roll_grades_s() {
        let damage = rolled_attribute(DAMAGE_TAG, BEST_ROLL, false);
        assert!(
            (damage.percentile - 1.0).abs() < 1e-9,
            "{}",
            damage.percentile
        );
        assert_eq!(damage.grade, "S");
        let (rolled, max) = (damage.rolled.unwrap(), damage.max.unwrap());
        assert!((rolled - max).abs() < 1e-9, "{rolled} {max}");
    }

    #[test]
    fn middle_roll_grades_b() {
        let damage = rolled_attribute(DAMAGE_TAG, MIDDLE_ROLL, false);
        assert!(
            (damage.percentile - 0.5).abs() < 1e-9,
            "{}",
            damage.percentile
        );
        assert_eq!(damage.grade, "B");
    }

    #[test]
    fn curse_grades_invert() {
        let weakest = rolled_attribute(DAMAGE_TAG, WORST_ROLL, true);
        assert!(
            (weakest.percentile - 1.0).abs() < 1e-9,
            "{}",
            weakest.percentile
        );
        assert_eq!(weakest.grade, "S");
        let (rolled, min, max) = (
            weakest.rolled.unwrap(),
            weakest.min.unwrap(),
            weakest.max.unwrap(),
        );
        assert!(rolled < 0.0 && min < max);
        assert!((rolled - max).abs() < 1e-9, "{rolled} {max}");

        let strongest = rolled_attribute(DAMAGE_TAG, BEST_ROLL, true);
        assert!(
            strongest.percentile.abs() < 1e-9,
            "{}",
            strongest.percentile
        );
        assert_eq!(strongest.grade, "F");
    }

    #[test]
    fn fixture_letters_match_percentile() {
        let tab = riven_tab();
        for row in &tab.unveiled {
            for attribute in &row.attributes {
                assert_eq!(
                    attribute.grade,
                    letter(attribute.percentile),
                    "{} {}",
                    row.item_id,
                    attribute.tag
                );
            }
        }
        let letters: Vec<&str> = tab
            .unveiled
            .iter()
            .flat_map(|row| row.attributes.iter())
            .map(|attribute| attribute.grade)
            .collect();
        assert!(letters.contains(&"A+"));
        assert!(
            letters
                .iter()
                .all(|band| *band == "S" || GRADE_BANDS.iter().any(|(_, known)| known == band))
        );
    }

    #[test]
    fn game_float_equivalence() {
        let alike =
            |left: i64, right: i64| game_float(left).to_bits() == game_float(right).to_bits();
        assert!(alike(708_669_601, 708_669_663));
        assert!(alike(903_450_000, 903_450_016));
        assert!(!alike(708_669_601, 741_320_000));
        assert!(!alike(1, 2));
    }

    #[test]
    fn fingerprint_weapon_path() {
        assert_eq!(
            fingerprint_weapon(CURRENT_ROLL).as_deref(),
            Some("/Lotus/Weapons/Archon/Melee/DualDaggers/ArchonDualDaggersPlayerWep")
        );
        assert_eq!(fingerprint_weapon("{"), None);
    }

    #[test]
    fn renamed_compat_paths() {
        assert_eq!(
            compat_path("/Lotus/Weapons/Tenno/Melee/Dagger/DarkDaggerBase"),
            "/Lotus/Weapons/Tenno/Melee/Dagger/DarkDagger"
        );
        assert_eq!(
            compat_path("/Lotus/Weapons/Tenno/Shotgun/QuadShotgunBase"),
            "/Lotus/Weapons/Tenno/Shotgun/QuadShotgun"
        );
        assert_eq!(
            compat_path("/Lotus/Weapons/Tenno/Rifle/Soma"),
            "/Lotus/Weapons/Tenno/Rifle/Soma"
        );
    }
}
