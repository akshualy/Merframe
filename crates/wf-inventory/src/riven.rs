use serde::Deserialize;

use crate::{InventoryError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RivenStat {
    #[serde(rename = "Tag")]
    pub tag: String,
    #[serde(rename = "Value")]
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RivenChallenge {
    #[serde(rename = "Type")]
    pub challenge_type: String,
    #[serde(default, rename = "Progress")]
    pub progress: i64,
    #[serde(default, rename = "Required")]
    pub required: i64,
    #[serde(rename = "Complication")]
    pub complication: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RivenFingerprint {
    pub compat: Option<String>,
    pub challenge: Option<RivenChallenge>,
    pub lim: Option<i64>,
    #[serde(default)]
    pub lvl: u32,
    #[serde(rename = "lvlReq")]
    pub lvl_req: Option<u32>,
    #[serde(default)]
    pub rerolls: u32,
    pub pol: Option<String>,
    #[serde(default)]
    pub buffs: Vec<RivenStat>,
    #[serde(default)]
    pub curses: Vec<RivenStat>,
}

impl RivenFingerprint {
    pub fn parse(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(InventoryError::from)
    }

    pub fn is_unveiled(&self) -> bool {
        self.compat.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VEILED: &str = r#"{"challenge":{"Type":"/Lotus/Types/Challenges/RandomizedKill","Progress":0,"Required":345,"Complication":"/Lotus/Types/Challenges/Complications/ResetOnNewDay"},"lvl":0}"#;

    const UNVEILED: &str = r#"{"compat":"/Lotus/Weapons/Tenno/Bows/PrimeDerelictCernos/DerelictCernos","lim":416959779,"lvlReq":14,"rerolls":60,"pol":"AP_ATTACK","buffs":[{"Tag":"WeaponSlashDamageMod","Value":867166414},{"Tag":"WeaponStunChanceMod","Value":333538354},{"Tag":"WeaponProcTimeMod","Value":739609447}],"curses":[{"Tag":"WeaponReloadSpeedMod","Value":929918906}],"lvl":8}"#;

    #[test]
    fn unveiled_riven() {
        let fp = RivenFingerprint::parse(UNVEILED).unwrap();
        assert_eq!(
            fp.compat.as_deref(),
            Some("/Lotus/Weapons/Tenno/Bows/PrimeDerelictCernos/DerelictCernos")
        );
        assert_eq!(fp.lim, Some(416_959_779));
        assert_eq!(fp.lvl, 8);
        assert_eq!(fp.lvl_req, Some(14));
        assert_eq!(fp.rerolls, 60);
        assert_eq!(fp.pol.as_deref(), Some("AP_ATTACK"));
        assert_eq!(fp.buffs.len(), 3);
        assert_eq!(fp.buffs[0].tag, "WeaponSlashDamageMod");
        assert_eq!(fp.buffs[0].value, 867_166_414);
        assert_eq!(fp.curses.len(), 1);
        assert_eq!(fp.curses[0].tag, "WeaponReloadSpeedMod");
        assert!(fp.is_unveiled());
    }

    #[test]
    fn veiled_riven() {
        let fp = RivenFingerprint::parse(VEILED).unwrap();
        assert!(!fp.is_unveiled());
        assert_eq!(fp.lvl, 0);
        assert!(fp.buffs.is_empty());
        assert!(fp.curses.is_empty());
        let challenge = fp.challenge.unwrap();
        assert_eq!(
            challenge.challenge_type,
            "/Lotus/Types/Challenges/RandomizedKill"
        );
        assert_eq!(challenge.progress, 0);
        assert_eq!(challenge.required, 345);
        assert_eq!(
            challenge.complication.as_deref(),
            Some("/Lotus/Types/Challenges/Complications/ResetOnNewDay")
        );
    }

    #[test]
    fn plain_mod() {
        let fp = RivenFingerprint::parse(r#"{"lvl":5}"#).unwrap();
        assert_eq!(fp.lvl, 5);
        assert_eq!(fp.rerolls, 0);
    }
}
