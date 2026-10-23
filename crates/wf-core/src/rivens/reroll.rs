use wf_data::RivenType;
use wf_inventory::RivenFingerprint;

use super::grading::{compat_path, perfectness, polarity, riven_name, roll_grade, same_roll};
use super::listing::unlisted_stat;
use super::{Grader, PendingRoll, RivenRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptRoll {
    Current,
    Offered,
}

pub fn kept_roll(row: &RivenRow, fingerprint: &str) -> Option<KeptRoll> {
    let offered = row.pending.as_ref()?;
    let kept = RivenFingerprint::parse(fingerprint).ok()?;
    if kept.compat.as_deref().map(compat_path) != row.weapon_path.as_deref() {
        return None;
    }
    if same_roll(&offered.attributes, &kept) {
        return Some(KeptRoll::Offered);
    }
    same_roll(&row.attributes, &kept).then_some(KeptRoll::Current)
}

impl Grader<'_> {
    pub(super) fn pending_from(
        &self,
        fingerprint: &RivenFingerprint,
        riven_type: Option<&RivenType>,
        disposition: Option<f64>,
    ) -> PendingRoll {
        let good = self.good_roll_of(fingerprint.compat.as_deref());
        let graded = self.graded(fingerprint, riven_type, disposition, good);
        let grade = perfectness(&graded);
        PendingRoll {
            name: riven_type.and_then(|found| riven_name(found, &fingerprint.buffs)),
            rerolls: fingerprint.rerolls,
            polarity: fingerprint.pol.as_deref().and_then(polarity),
            grade,
            grade_letter: roll_grade(grade),
            good_roll: good.map(|good| self.good_roll(good, &graded, riven_type)),
            splices: riven_type
                .map_or_else(Vec::new, |found| self.open_splice_names(found, &graded)),
            attributes: graded,
        }
    }

    pub(crate) fn cycled(&self, row: &RivenRow, current: &str, pending: &str) -> Option<RivenRow> {
        let current = RivenFingerprint::parse(current).ok()?;
        let pending = RivenFingerprint::parse(pending).ok()?;
        let riven_type = self.riven_type(&row.item_type);
        let shown = self.pending_from(&current, riven_type, row.disposition);
        Some(RivenRow {
            pending: Some(self.pending_from(&pending, riven_type, row.disposition)),
            ..row.clone().with_roll(shown)
        })
    }
}

impl RivenRow {
    pub(super) fn with_roll(self, roll: PendingRoll) -> Self {
        Self {
            name: roll.name,
            rerolls: roll.rerolls,
            polarity: roll.polarity,
            grade: roll.grade,
            grade_letter: roll.grade_letter,
            unlisted_stat: unlisted_stat(&roll.attributes),
            attributes: roll.attributes,
            splices: roll.splices,
            good_roll: roll.good_roll,
            pending: None,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rivens::AttributeGrade;
    use crate::rivens::support::*;

    fn cycled_nepheri(current: &str, pending: &str) -> RivenRow {
        let tab = riven_tab();
        let row = by_weapon(&tab.unveiled, "Nepheri");
        assert_eq!(row.rerolls, 74);
        fixture().grader().cycled(row, current, pending).unwrap()
    }

    fn tags(attributes: &[AttributeGrade]) -> Vec<&str> {
        attributes
            .iter()
            .map(|attribute| attribute.tag.as_str())
            .collect()
    }

    #[test]
    fn current_and_pending_roll() {
        let row = cycled_nepheri(&paid_roll(), PENDING_ROLL);

        assert_eq!(row.item_id, "f29623082864e554d187a3f7");
        assert_eq!(row.rerolls, 75);
        assert_eq!(row.name.as_deref(), Some("Geli-toxidra"));
        assert_eq!(
            tags(&row.attributes),
            [
                "WeaponFreezeDamageMod",
                "WeaponToxinDamageMod",
                "WeaponFireRateMod"
            ]
        );

        let pending = row.pending.unwrap();
        assert_eq!(pending.rerolls, 75);
        assert_eq!(pending.name.as_deref(), Some("Acri-gelipha"));
        assert_eq!(pending.polarity, row.polarity);
        assert_eq!(
            tags(&pending.attributes),
            [
                "WeaponFreezeDamageMod",
                "WeaponCritDamageMod",
                "WeaponFireDamageMod",
                "SlideAttackCritChanceMod"
            ]
        );
        assert_eq!(
            pending.attributes.iter().filter(|stat| stat.curse).count(),
            1
        );
        assert!(pending.attributes.iter().all(|stat| stat.display.is_some()));
        assert!(pending.grade > 0.0);
    }

    #[test]
    fn kept_roll_after_cycle() {
        let row = cycled_nepheri(PENDING_ROLL, &paid_roll());

        assert_eq!(row.name.as_deref(), Some("Acri-gelipha"));
        assert_eq!(
            tags(&row.attributes),
            [
                "WeaponFreezeDamageMod",
                "WeaponCritDamageMod",
                "WeaponFireDamageMod",
                "SlideAttackCritChanceMod"
            ]
        );
        assert_eq!(
            row.pending.map(|pending| pending.name),
            Some(Some(String::from("Geli-toxidra")))
        );
    }

    #[test]
    fn kept_station_roll() {
        let paid = paid_roll();
        let row = cycled_nepheri(&paid, PENDING_ROLL);

        assert_eq!(kept_roll(&row, &paid), Some(KeptRoll::Current));
        assert_eq!(kept_roll(&row, PENDING_ROLL), Some(KeptRoll::Offered));
        assert_eq!(
            kept_roll(
                &row,
                &PENDING_ROLL.replace("\"rerolls\":75", "\"rerolls\":76")
            ),
            Some(KeptRoll::Offered)
        );

        let elsewhere = paid.replace(ARCHON_DAGGERS, "/Lotus/Weapons/Tenno/Rifle/Rifle");
        assert_eq!(kept_roll(&row, &elsewhere), None);
        assert_eq!(kept_roll(&row, "not json"), None);
        assert_eq!(
            kept_roll(&row, &paid.replace("708669601", "903450000")),
            None
        );

        let tab = riven_tab();
        assert_eq!(
            kept_roll(by_weapon(&tab.unveiled, "Nepheri"), CURRENT_ROLL),
            None
        );
    }

    #[test]
    fn garbage_fingerprint() {
        let tab = riven_tab();
        let row = by_weapon(&tab.unveiled, "Nepheri");
        let fixture = fixture();
        let grader = fixture.grader();
        assert!(grader.cycled(row, "{", PENDING_ROLL).is_none());
        assert!(grader.cycled(row, CURRENT_ROLL, "").is_none());
    }

    #[test]
    fn splice_options_until_spliced() {
        let tab = riven_tab();
        let row = by_weapon(&tab.unveiled, "Nepheri");
        let fixture = fixture();
        let grader = fixture.grader();
        let weapon = row.weapon_path.as_deref().unwrap();
        let elements = rolled(
            weapon,
            &["WeaponToxinDamageMod", "WeaponFireDamageMod"],
            Some("WeaponFreezeDamageMod"),
        );
        let open = grader.cycled(row, &elements, &elements).unwrap();
        assert_eq!(open.splices, ["Gas", "Viral", "Blast"]);
        assert_eq!(open.pending.unwrap().splices, ["Gas", "Viral", "Blast"]);

        let spliced = rolled(
            weapon,
            &["WeaponToxinDamageMod", "WeaponGasDamageMod"],
            Some("WeaponFreezeDamageMod"),
        );
        let held = grader.cycled(row, &spliced, &elements).unwrap();
        assert!(held.splices.is_empty());
        assert!(held.attributes.iter().any(|attribute| attribute.spliced));
    }
}
