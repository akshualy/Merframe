use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Favourites {
    names: BTreeSet<String>,
}

impl Favourites {
    pub fn contains(&self, unique_name: &str) -> bool {
        self.names.contains(unique_name)
    }

    pub fn any<'a>(&self, unique_names: impl IntoIterator<Item = &'a str>) -> bool {
        unique_names.into_iter().any(|name| self.contains(name))
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

impl<T: Into<String>> FromIterator<T> for Favourites {
    fn from_iter<I: IntoIterator<Item = T>>(names: I) -> Self {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_and_any() {
        let favourites: Favourites = ["/Lotus/A", "/Lotus/B"].into_iter().collect();
        assert!(favourites.contains("/Lotus/A"));
        assert!(!favourites.contains("/Lotus/C"));
        assert!(favourites.any(["/Lotus/C", "/Lotus/B"]));
        assert!(!favourites.any(["/Lotus/C", "/Lotus/D"]));
        assert_eq!(favourites.len(), 2);
        assert!(!favourites.is_empty());
        assert_eq!(
            favourites.names().collect::<Vec<_>>(),
            ["/Lotus/A", "/Lotus/B"]
        );
        assert!(Favourites::default().is_empty());
    }

    #[test]
    fn sorted_json_array() {
        let favourites: Favourites = ["/Lotus/B", "/Lotus/A"].into_iter().collect();
        assert_eq!(
            serde_json::to_string(&favourites).unwrap(),
            r#"["/Lotus/A","/Lotus/B"]"#
        );
    }
}
