use crate::embedded::{CATCH_GRADES, CatchGrades, MISC_ITEM_NAMES, lookup};

pub fn misc_item_name(unique_name: &str) -> Option<&'static str> {
    lookup(MISC_ITEM_NAMES, unique_name).copied()
}

fn catch_grades(unique_name: &str) -> Option<&'static CatchGrades> {
    CATCH_GRADES
        .iter()
        .find(|grades| unique_name.starts_with(grades.prefix))
}

pub fn catch_size(unique_name: &str) -> Option<String> {
    let grades = catch_grades(unique_name)?;
    Some(match catch_grade(unique_name) {
        Some((_, grade)) => grade.to_ascii_lowercase(),
        None if grades.medium == "Medium" => "small".to_owned(),
        None => "basic".to_owned(),
    })
}

pub fn catch_grade(unique_name: &str) -> Option<(String, &'static str)> {
    let grades = catch_grades(unique_name)?;
    for (marker, grade) in [("Large", grades.large), ("Medium", grades.medium)] {
        if let Some(base) = unique_name.strip_suffix(marker) {
            return Some((base.to_owned(), grade));
        }
        if let Some(base) = unique_name.strip_suffix(&format!("{marker}Item")) {
            return Some((format!("{base}Item"), grade));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_tables_sorted() {
        assert!(MISC_ITEM_NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(!CATCH_GRADES.is_empty());
    }

    #[test]
    fn curated_names() {
        assert_eq!(
            misc_item_name("/Lotus/Types/Keys/Nightwave/GlassmakerBossFightKey"),
            Some("Enter Nihil's Oubliette")
        );
        assert_eq!(
            misc_item_name("/Lotus/Upgrades/Skins/Horse/HorseHelmetDrapery"),
            Some("Aumen Chamfron")
        );
        assert_eq!(misc_item_name("/Lotus/Types/Items/MiscItems/Ferrite"), None);
    }

    #[test]
    fn size_graded_catches() {
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItemLarge"),
            Some((
                "/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem".to_owned(),
                "Large"
            ))
        );
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Deimos/InfestedCommonDFishItemMedium"),
            Some((
                "/Lotus/Types/Items/Fish/Deimos/InfestedCommonDFishItem".to_owned(),
                "Medium"
            ))
        );
    }

    #[test]
    fn quality_graded_catches() {
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Solaris/OrokinCoolRareFishALargeItem"),
            Some((
                "/Lotus/Types/Items/Fish/Solaris/OrokinCoolRareFishAItem".to_owned(),
                "Magnificent"
            ))
        );
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAMediumItem"),
            Some((
                "/Lotus/Types/Items/Fish/Solaris/SolarisCoolCommonFishAItem".to_owned(),
                "Adorned"
            ))
        );
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Deimos/HybridRareAFishItemLarge"),
            Some((
                "/Lotus/Types/Items/Fish/Deimos/HybridRareAFishItem".to_owned(),
                "Magnificent"
            ))
        );
    }

    #[test]
    fn catch_sizes_are_market_subtypes() {
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem"),
            Some("small".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItemMedium"),
            Some("medium".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Deimos/InfestedCommonDFishItemLarge"),
            Some("large".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Solaris/OrokinCoolRareFishAItem"),
            Some("basic".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Solaris/OrokinCoolRareFishAMediumItem"),
            Some("adorned".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Deimos/HybridRareAFishItemLarge"),
            Some("magnificent".to_owned())
        );
        assert_eq!(
            catch_size("/Lotus/Types/Items/Fish/Duviri/DuviriFishAItem"),
            None
        );
    }

    #[test]
    fn ungraded_catch() {
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Duviri/DuviriFishAItem"),
            None
        );
        assert_eq!(
            catch_grade("/Lotus/Types/Items/Fish/Eidolon/DayUncommonFishBItem"),
            None
        );
        assert_eq!(
            catch_grade("/Lotus/Types/Items/MiscItems/OrokinCellLarge"),
            None
        );
    }
}
