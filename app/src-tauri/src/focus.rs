use std::sync::RwLock;

use crate::state::{read, write};

#[derive(Debug, Default)]
pub struct GameFocus(RwLock<Option<bool>>);

impl GameFocus {
    pub fn changed(&self, focused: bool) {
        *write(&self.0) = Some(focused);
    }

    pub fn forget(&self) {
        *write(&self.0) = None;
    }

    pub fn focused(&self) -> Option<bool> {
        *read(&self.0)
    }

    pub fn suppresses(&self, only_background: bool) -> bool {
        only_background && self.focused() == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use wf_log::{Event, classify, read_all};

    use super::*;

    fn fixture_focus_changes() -> Vec<bool> {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/EE.log"
        ));
        read_all(path)
            .unwrap()
            .iter()
            .filter_map(|line| match classify(line) {
                Some(Event::WindowFocus { focused }) => Some(focused),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn unknown_focus() {
        let focus = GameFocus::default();
        assert_eq!(focus.focused(), None);
        assert!(!focus.suppresses(true));
    }

    #[test]
    fn fixture_changes_tracked() {
        let changes = fixture_focus_changes();
        assert_eq!(changes.len(), 162);
        assert!(changes.contains(&true));
        assert!(changes.contains(&false));

        let focus = GameFocus::default();
        for (index, focused) in changes.iter().enumerate() {
            focus.changed(*focused);
            assert_eq!(focus.focused(), Some(*focused), "change {index}");
            assert_eq!(focus.suppresses(true), *focused, "change {index}");
        }
    }

    #[test]
    fn forget_on_game_exit() {
        let focus = GameFocus::default();
        for focused in fixture_focus_changes() {
            focus.changed(focused);
        }
        focus.forget();
        assert_eq!(focus.focused(), None);
        assert!(!focus.suppresses(true));
    }

    #[test]
    fn suppresses_only_when_focused() {
        let focus = GameFocus::default();
        assert!(!focus.suppresses(true));
        assert!(!focus.suppresses(false));
        focus.changed(false);
        assert!(!focus.suppresses(true));
        assert!(!focus.suppresses(false));
        focus.changed(true);
        assert!(focus.suppresses(true));
        assert!(!focus.suppresses(false));
    }
}
