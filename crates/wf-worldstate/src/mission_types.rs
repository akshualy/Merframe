use crate::embedded::{MISSION_TYPES, lookup};

pub fn mission_type_name(id: &str) -> String {
    known_or_title_case(MISSION_TYPES, id, "MT_", |name| name)
}

pub fn known_or_title_case<T>(
    table: &[(&str, T)],
    id: &str,
    prefix: &str,
    name: impl FnOnce(&T) -> &str,
) -> String {
    lookup(table, id).map_or_else(
        || title_case_key(id.strip_prefix(prefix).unwrap_or(id)),
        |entry| name(entry).to_owned(),
    )
}

pub fn title_case_key(key: &str) -> String {
    key.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            let mut titled: String = chars.by_ref().take(1).collect();
            titled.extend(chars.flat_map(char::to_lowercase));
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_sorted() {
        assert!(MISSION_TYPES.is_sorted_by(|a, b| a.0 < b.0));
    }

    #[test]
    fn known_mission_type() {
        assert_eq!(mission_type_name("MT_EXTERMINATION"), "Extermination");
    }

    #[test]
    fn disruption_alias() {
        assert_eq!(mission_type_name("MT_ARTIFACT"), "Disruption");
    }

    #[test]
    fn unmapped_id_is_title_cased() {
        assert_eq!(
            mission_type_name("MT_NOT_A_REAL_MISSION"),
            "Not A Real Mission"
        );
        assert_eq!(mission_type_name("SOMETHING_ELSE"), "Something Else");
    }

    #[test]
    fn title_cased_keys() {
        assert_eq!(title_case_key("SHOTGUN_ONLY"), "Shotgun Only");
        assert_eq!(title_case_key("KELA"), "Kela");
        assert_eq!(title_case_key(""), "");
    }

    #[test]
    fn default_entry() {
        assert_eq!(mission_type_name("MT_DEFAULT"), "Unknown");
    }
}
