#[cfg(not(target_os = "windows"))]
use std::path::Path;
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
fn parse_vdf_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let value = line
                .trim()
                .strip_prefix("\"path\"")?
                .trim()
                .trim_matches('"');
            if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            }
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn steam_default_roots() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    vec![
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ]
}

#[cfg(not(target_os = "windows"))]
fn steam_library_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for root in steam_default_roots() {
        let vdf_path = root.join("steamapps/libraryfolders.vdf");
        if let Ok(contents) = std::fs::read_to_string(&vdf_path) {
            for library in parse_vdf_library_paths(&contents) {
                if !roots.contains(&library) {
                    roots.push(library);
                }
            }
        }
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

#[cfg(not(target_os = "windows"))]
fn warframe_prefix_dir(steam_library: &Path) -> PathBuf {
    steam_library
        .join("steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe")
}

const LOG_FILE: &str = "EE.log";

#[cfg(target_os = "windows")]
fn default_log_dir() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local_app_data).join("Warframe"))
}

#[cfg(not(target_os = "windows"))]
fn prefix_with_log(prefixes: &[PathBuf]) -> Option<PathBuf> {
    prefixes
        .iter()
        .find(|prefix| prefix.join(LOG_FILE).is_file())
        .or_else(|| prefixes.iter().find(|prefix| prefix.is_dir()))
        .cloned()
}

#[cfg(not(target_os = "windows"))]
fn default_log_dir() -> Option<PathBuf> {
    let prefixes: Vec<PathBuf> = steam_library_roots()
        .iter()
        .map(|library| warframe_prefix_dir(library))
        .collect();
    prefix_with_log(&prefixes)
}

pub fn default_log_path() -> Option<PathBuf> {
    Some(default_log_dir()?.join(LOG_FILE))
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn vdf_library_paths() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/data/SteamLibrary"
		"label"		""
		"contentid"		"123"
	}
	"1"
	{
		"path"		"/home/tenno/.steam/steam"
	}
}
"#;
        assert_eq!(
            parse_vdf_library_paths(vdf),
            vec![
                PathBuf::from("/data/SteamLibrary"),
                PathBuf::from("/home/tenno/.steam/steam"),
            ]
        );
    }

    #[test]
    fn vdf_without_paths() {
        assert!(parse_vdf_library_paths("\"libraryfolders\"\n{\n}\n").is_empty());
    }

    #[test]
    fn prefix_path() {
        let library = Path::new("/data/SteamLibrary");
        assert_eq!(
            warframe_prefix_dir(library),
            PathBuf::from(
                "/data/SteamLibrary/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe"
            )
        );
    }

    #[test]
    fn live_prefix() {
        let root = std::env::temp_dir().join(format!("wf-log-paths-test-{}", std::process::id()));
        let stale = root.join("stale");
        let live = root.join("live");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::create_dir_all(&live).unwrap();
        std::fs::write(live.join(LOG_FILE), b"").unwrap();

        let missing = root.join("missing");
        assert_eq!(prefix_with_log(&[]), None);
        assert_eq!(prefix_with_log(std::slice::from_ref(&missing)), None);
        assert_eq!(
            prefix_with_log(&[missing, stale.clone()]),
            Some(stale.clone())
        );
        assert_eq!(prefix_with_log(&[stale, live.clone()]), Some(live));

        std::fs::remove_dir_all(&root).unwrap();
    }
}
