pub fn basename(path: &str) -> &str {
    match path.rfind(['\\', '/']) {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

pub fn basename_matches(path: &str, name: &str) -> bool {
    basename(path).eq_ignore_ascii_case(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_argv0() {
        assert!(basename_matches(
            "Z:\\x\\Warframe.x64.exe",
            "warframe.x64.exe"
        ));
    }

    #[test]
    fn unix_path_and_bare_name() {
        assert!(basename_matches(
            "/media/test/SteamLibrary/steamapps/common/Warframe/Warframe.x64.exe",
            "Warframe.x64.exe"
        ));
        assert!(basename_matches("Warframe.x64.exe", "WARFRAME.X64.EXE"));
    }

    #[test]
    fn look_alike_name() {
        assert!(!basename_matches(
            "Z:\\x\\NotWarframe.x64.exe",
            "Warframe.x64.exe"
        ));
        assert!(!basename_matches("Z:\\x\\Warframe.x64.exe", ".x64.exe"));
        assert!(!basename_matches(
            "Z:\\x\\Warframe.x64.exe.bak",
            "Warframe.x64.exe"
        ));
    }

    #[test]
    fn directory_component() {
        assert!(!basename_matches(
            "Z:\\Warframe.x64.exe\\Tools\\Launcher.exe",
            "Warframe.x64.exe"
        ));
        assert!(!basename_matches("Z:\\x\\Warframe.x64.exe", "launcher.exe"));
        assert!(!basename_matches("x64.exe", "Warframe.x64.exe"));
    }

    #[test]
    fn mixed_separators() {
        assert_eq!(basename("Z:\\a/b\\c.exe"), "c.exe");
        assert_eq!(basename("plain"), "plain");
        assert_eq!(basename("[heap]"), "[heap]");
    }
}
