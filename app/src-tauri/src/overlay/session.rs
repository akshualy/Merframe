#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Serialize;

use crate::settings::OverlayMode;

pub const FORCE_ENV: &str = "MERFRAME_OVERLAY_WINDOWS";
pub const XWAYLAND_ENV: &str = "MERFRAME_OVERLAY_XWAYLAND";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Session {
    Windows,
    LinuxX11,
    LinuxXWayland,
    LinuxWayland,
    LinuxWaylandRestart,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Startup {
    pub session: Session,
    pub x_display: Option<u32>,
    pub gdk_backend_set: bool,
    pub overlays_wanted: bool,
    pub adopted: bool,
}

static STARTUP: OnceLock<Startup> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OverlaySupport {
    pub windows_possible: bool,
    pub detail: String,
}

pub fn session_of(
    os: &str,
    session_type: Option<&str>,
    wayland_display: Option<&str>,
    x11: bool,
) -> Session {
    match os {
        "windows" => Session::Windows,
        "linux" => {
            let wayland = session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
                || wayland_display.is_some_and(|value| !value.is_empty());
            match (wayland, x11) {
                (true, true) => Session::LinuxXWayland,
                (true, false) => Session::LinuxWayland,
                (false, _) => Session::LinuxX11,
            }
        }
        _ => Session::Other,
    }
}

pub fn x_display_number(display: &str) -> Option<u32> {
    let (host, tail) = display.rsplit_once(':')?;
    if !host.is_empty() && host != "unix" {
        return None;
    }
    tail.split_once('.')
        .map_or(tail, |(number, _)| number)
        .parse()
        .ok()
}

#[cfg(target_os = "linux")]
pub fn x_display_listening(number: u32) -> bool {
    std::os::unix::net::UnixStream::connect(format!("/tmp/.X11-unix/X{number}")).is_ok()
}

impl Startup {
    pub fn adopts_x11(&self, xwayland_env: Option<&str>, forced: bool) -> bool {
        self.session == Session::LinuxWayland
            && self.x_display.is_some()
            && !self.gdk_backend_set
            && xwayland_env != Some("0")
            && (xwayland_env == Some("1") || forced || self.overlays_wanted)
    }
}

fn session_from_env(x11: bool) -> (Session, bool) {
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let forced = std::env::var(FORCE_ENV).is_ok_and(|value| !value.is_empty() && value != "0");
    let session = session_of(
        std::env::consts::OS,
        session_type.as_deref(),
        wayland_display.as_deref(),
        x11,
    );
    (session, forced)
}

#[cfg(target_os = "linux")]
pub fn adopt_xwayland() -> Startup {
    let (session, forced) = session_from_env(false);
    let x_display = std::env::var("DISPLAY")
        .ok()
        .and_then(|display| x_display_number(&display))
        .filter(|number| x_display_listening(*number));
    let gdk_backend_set = std::env::var("GDK_BACKEND").is_ok_and(|value| !value.is_empty());
    let xwayland_env = std::env::var(XWAYLAND_ENV).ok();
    let document = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|base| {
            base.join(crate::state::DATA_DIR)
                .join(crate::settings::STORE_FILE)
        })
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_default();
    let startup = Startup {
        session,
        x_display,
        gdk_backend_set,
        overlays_wanted: crate::settings::wants_overlay_windows(&document),
        adopted: false,
    };
    let adopted = startup.adopts_x11(xwayland_env.as_deref(), forced);
    if adopted {
        gdk::set_allowed_backends("x11");
    }
    *STARTUP.get_or_init(|| Startup { adopted, ..startup })
}

pub fn support_for(session: Session, forced: bool) -> OverlaySupport {
    let (windows_possible, detail) = match session {
        Session::Windows => (
            true,
            "On Windows the overlays stay above the game and let clicks through.".to_owned(),
        ),
        Session::LinuxX11 => (
            true,
            "On X11 the overlays stay above the game and let clicks through.".to_owned(),
        ),
        Session::LinuxXWayland => (
            true,
            "On Wayland the overlays run through XWayland, next to the game.".to_owned(),
        ),
        Session::LinuxWayland | Session::LinuxWaylandRestart if forced => (
            true,
            format!(
                "Overlay windows are forced on by {FORCE_ENV}. The compositor may still ignore their position and the always-on-top hint."
            ),
        ),
        Session::LinuxWaylandRestart => (
            false,
            "The overlays move onto XWayland when Merframe restarts. Until then they render in this tab."
                .to_owned(),
        ),
        Session::LinuxWayland => (
            false,
            format!(
                "This Wayland session has no X server the overlays could share with the game. Set {FORCE_ENV}=1 to try anyway."
            ),
        ),
        Session::Other => (
            false,
            "This platform has no overlay windows.".to_owned(),
        ),
    };
    OverlaySupport {
        windows_possible,
        detail,
    }
}

pub fn probe() -> OverlaySupport {
    let startup = STARTUP.get().copied().unwrap_or_default();
    let (session, forced) = session_from_env(startup.adopted);
    let session = match session {
        Session::LinuxWayland if startup.x_display.is_some() => Session::LinuxWaylandRestart,
        session => session,
    };
    support_for(session, forced)
}

pub fn uses_windows(mode: OverlayMode, support: &OverlaySupport) -> bool {
    match mode {
        OverlayMode::Auto => support.windows_possible,
        OverlayMode::Windows => true,
        OverlayMode::Tab => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::OverlayMode;

    #[test]
    fn session_support_matrix() {
        assert_eq!(session_of("windows", None, None, false), Session::Windows);
        assert_eq!(
            session_of("linux", Some("x11"), None, false),
            Session::LinuxX11
        );
        assert_eq!(session_of("linux", None, None, false), Session::LinuxX11);
        assert_eq!(
            session_of("linux", Some("wayland"), None, false),
            Session::LinuxWayland
        );
        assert_eq!(
            session_of("linux", Some("wayland"), None, true),
            Session::LinuxXWayland
        );
        assert_eq!(
            session_of("linux", Some("Wayland"), None, false),
            Session::LinuxWayland
        );
        assert_eq!(
            session_of("linux", Some("tty"), Some("wayland-0"), false),
            Session::LinuxWayland
        );
        assert_eq!(
            session_of("linux", Some("x11"), Some(""), false),
            Session::LinuxX11
        );
        assert_eq!(session_of("macos", None, None, false), Session::Other);

        assert!(support_for(Session::Windows, false).windows_possible);
        assert!(support_for(Session::LinuxX11, false).windows_possible);
        assert!(!support_for(Session::Other, true).windows_possible);

        let xwayland = support_for(Session::LinuxXWayland, false);
        assert!(xwayland.windows_possible);
        assert!(
            xwayland
                .detail
                .starts_with("On Wayland the overlays run through XWayland")
        );

        let restart = support_for(Session::LinuxWaylandRestart, false);
        assert!(!restart.windows_possible);
        assert!(
            restart
                .detail
                .starts_with("The overlays move onto XWayland when Merframe restarts")
        );
        assert!(support_for(Session::LinuxWaylandRestart, true).windows_possible);

        let wayland = support_for(Session::LinuxWayland, false);
        assert!(!wayland.windows_possible);
        assert!(
            wayland
                .detail
                .starts_with("This Wayland session has no X server")
        );
        assert!(support_for(Session::LinuxWayland, true).windows_possible);
    }

    #[test]
    fn local_x_display_number() {
        assert_eq!(x_display_number(":0"), Some(0));
        assert_eq!(x_display_number(":1.0"), Some(1));
        assert_eq!(x_display_number("unix:0"), Some(0));
        assert_eq!(x_display_number(":0.x"), Some(0));
        assert_eq!(x_display_number(""), None);
        assert_eq!(x_display_number(":"), None);
        assert_eq!(x_display_number(":abc"), None);
        assert_eq!(x_display_number("localhost:10.0"), None);
        assert_eq!(x_display_number("somehost:0"), None);
    }

    #[test]
    fn xwayland_adoption() {
        let wayland = Startup {
            session: Session::LinuxWayland,
            x_display: Some(0),
            gdk_backend_set: false,
            overlays_wanted: true,
            adopted: false,
        };
        assert!(wayland.adopts_x11(None, false));
        assert!(wayland.adopts_x11(None, true));
        assert!(wayland.adopts_x11(Some("1"), false));
        assert!(!wayland.adopts_x11(Some("0"), true));

        let unwanted = Startup {
            overlays_wanted: false,
            ..wayland
        };
        assert!(!unwanted.adopts_x11(None, false));
        assert!(unwanted.adopts_x11(None, true));
        assert!(unwanted.adopts_x11(Some("1"), false));

        let no_x = Startup {
            x_display: None,
            ..wayland
        };
        assert!(!no_x.adopts_x11(None, true));

        let gdk_pinned = Startup {
            gdk_backend_set: true,
            ..wayland
        };
        assert!(!gdk_pinned.adopts_x11(None, true));

        let x11 = Startup {
            session: Session::LinuxX11,
            ..wayland
        };
        assert!(!x11.adopts_x11(Some("1"), true));
        let windows = Startup {
            session: Session::Windows,
            ..wayland
        };
        assert!(!windows.adopts_x11(Some("1"), true));
    }

    #[test]
    fn uses_windows_per_mode() {
        let yes = OverlaySupport {
            windows_possible: true,
            detail: String::new(),
        };
        let no = OverlaySupport::default();
        assert!(uses_windows(OverlayMode::Auto, &yes));
        assert!(!uses_windows(OverlayMode::Auto, &no));
        assert!(uses_windows(OverlayMode::Windows, &no));
        assert!(!uses_windows(OverlayMode::Tab, &yes));
    }
}
