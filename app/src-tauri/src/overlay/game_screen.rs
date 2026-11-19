use std::cmp::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tracing::{debug, warn};

use super::update_slots;
use super::window::Bounds;
use crate::settings::OverlayMonitorOrigin;
use crate::state::{AppState, read};

const DEFAULT_SCREEN: ScaledBounds = ScaledBounds {
    bounds: Bounds {
        width: 1920.0,
        height: 1080.0,
        x: 0.0,
        y: 0.0,
    },
    scale: 1.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ScaledBounds {
    pub(super) bounds: Bounds,
    pub(super) scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS), ts(export))]
pub struct OverlayMonitor {
    pub origin: OverlayMonitorOrigin,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
    pub chosen: bool,
}

impl OverlayMonitor {
    fn bounds(self) -> Bounds {
        Bounds {
            width: f64::from(self.width),
            height: f64::from(self.height),
            x: f64::from(self.origin.x),
            y: f64::from(self.origin.y),
        }
    }

    fn screen(self) -> ScaledBounds {
        ScaledBounds {
            bounds: self.bounds(),
            scale: self.scale,
        }
    }
}

fn by_area(one: &Bounds, other: &Bounds) -> Ordering {
    one.area().total_cmp(&other.area())
}

fn pick_screen(monitors: &[OverlayMonitor], window: Option<Bounds>) -> ScaledBounds {
    let game = window.and_then(|rect| {
        monitors
            .iter()
            .filter_map(|monitor| {
                rect.overlap(monitor.bounds())
                    .map(|part| (part, monitor.scale))
            })
            .max_by(|(one, _), (other, _)| by_area(one, other))
            .map(|(bounds, scale)| ScaledBounds { bounds, scale })
    });
    monitors
        .iter()
        .find(|monitor| monitor.chosen)
        .map(|monitor| monitor.screen())
        .or(game)
        .or_else(|| {
            monitors
                .iter()
                .find(|monitor| monitor.primary)
                .map(|monitor| monitor.screen())
        })
        .unwrap_or(DEFAULT_SCREEN)
}

fn monitors<R: Runtime>(
    app: &AppHandle<R>,
    chosen: Option<OverlayMonitorOrigin>,
) -> Vec<OverlayMonitor> {
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| *monitor.position());
    match app.available_monitors() {
        Ok(monitors) => monitors
            .iter()
            .map(|monitor| {
                let origin = OverlayMonitorOrigin {
                    x: monitor.position().x,
                    y: monitor.position().y,
                };
                OverlayMonitor {
                    origin,
                    width: monitor.size().width,
                    height: monitor.size().height,
                    scale: monitor.scale_factor(),
                    primary: primary == Some(*monitor.position()),
                    chosen: chosen == Some(origin),
                }
            })
            .collect(),
        Err(error) => {
            warn!(%error, "Monitor list unavailable");
            Vec::new()
        }
    }
}

pub(super) fn current_screen<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
) -> ScaledBounds {
    let chosen = read(&state.settings).overlays.overlay_monitor;
    let pid = read(&state.status).pid;
    let window = pid.and_then(game_window);
    let monitors = monitors(app, chosen);
    let screen = pick_screen(&monitors, window);

    debug!(?screen, ?window, ?chosen, "Overlay screen read");

    update_slots(app, state, |slots| {
        let changed = slots.monitors != monitors;
        slots.monitors = monitors;
        changed
    });

    screen
}

#[cfg(target_os = "linux")]
fn game_window(pid: u32) -> Option<Bounds> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, MapState};

    let (connection, screen) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots.get(screen)?.root;
    let atom = |name: &[u8]| Some(connection.intern_atom(true, name).ok()?.reply().ok()?.atom);
    let client_list = atom(b"_NET_CLIENT_LIST")?;
    let wm_pid = atom(b"_NET_WM_PID")?;
    let clients = connection
        .get_property(false, root, client_list, AtomEnum::WINDOW, 0, u32::MAX)
        .ok()?
        .reply()
        .ok()?;
    let pid_requests: Vec<_> = clients
        .value32()?
        .map(|window| {
            let cookie = connection.get_property(false, window, wm_pid, AtomEnum::CARDINAL, 0, 1);
            (window, cookie)
        })
        .collect();
    let rect = |window: u32| {
        let attributes = connection
            .get_window_attributes(window)
            .ok()?
            .reply()
            .ok()?;
        if attributes.map_state != MapState::VIEWABLE {
            return None;
        }

        let geometry = connection.get_geometry(window).ok()?.reply().ok()?;
        let origin = connection
            .translate_coordinates(window, root, 0, 0)
            .ok()?
            .reply()
            .ok()?;
        Some(Bounds {
            width: f64::from(geometry.width),
            height: f64::from(geometry.height),
            x: f64::from(origin.dst_x),
            y: f64::from(origin.dst_y),
        })
    };

    pid_requests
        .into_iter()
        .filter_map(|(window, cookie)| {
            let property = cookie.ok()?.reply().ok()?;
            let owned = property.value32()?.next() == Some(pid);
            owned.then_some(window)
        })
        .filter_map(rect)
        .max_by(by_area)
}

#[cfg(windows)]
fn game_window(pid: u32) -> Option<Bounds> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;
    use windows_sys::core::BOOL;

    unsafe extern "system" fn collect(window: HWND, windows: LPARAM) -> BOOL {
        // SAFETY: `windows` is the address of the vector EnumWindows was given below.
        unsafe { (*(windows as *mut Vec<HWND>)).push(window) };
        1
    }

    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: `collect` only pushes into `windows`.
    unsafe { EnumWindows(Some(collect), &raw mut windows as LPARAM) };
    windows
        .into_iter()
        .filter_map(|window| client_rect(window, pid))
        .max_by(by_area)
}

#[cfg(windows)]
fn client_rect(window: windows_sys::Win32::Foundation::HWND, pid: u32) -> Option<Bounds> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    };

    let mut owner = 0;
    // SAFETY: `window` came from EnumWindows and `owner` is a live local.
    unsafe { GetWindowThreadProcessId(window, &raw mut owner) };
    if owner != pid {
        return None;
    }

    let mut client = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: `window` and both pointers are live locals.
    let shown = unsafe {
        IsWindowVisible(window) != 0
            && IsIconic(window) == 0
            && GetClientRect(window, &raw mut client) != 0
            && ClientToScreen(window, &raw mut origin) != 0
    };
    shown.then(|| Bounds {
        width: f64::from(client.right),
        height: f64::from(client.bottom),
        x: f64::from(origin.x),
        y: f64::from(origin.y),
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn game_window(_: u32) -> Option<Bounds> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side_by_side() -> Vec<OverlayMonitor> {
        let monitor = |x, y, width, height, primary| OverlayMonitor {
            origin: OverlayMonitorOrigin { x, y },
            width,
            height,
            scale: 1.0,
            primary,
            chosen: false,
        };
        vec![
            monitor(0, 360, 1920, 1080, true),
            monitor(1920, 0, 2560, 1440, false),
        ]
    }

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            width,
            height,
            x,
            y,
        }
    }

    #[test]
    fn primary_without_game_window() {
        let monitors = side_by_side();
        assert_eq!(pick_screen(&monitors, None), monitors[0].screen());
        assert_eq!(pick_screen(&[], None), DEFAULT_SCREEN);
    }

    #[test]
    fn windowed_game_largest_overlap() {
        let straddling = rect(1600.0, 100.0, 1600.0, 900.0);
        assert_eq!(
            pick_screen(&side_by_side(), Some(straddling)),
            ScaledBounds {
                bounds: rect(1920.0, 100.0, 1280.0, 900.0),
                scale: 1.0,
            }
        );
    }

    #[test]
    fn chosen_monitor_wins() {
        let mut monitors = side_by_side();
        monitors[0].chosen = true;
        let fullscreen = rect(1920.0, 0.0, 2560.0, 1440.0);
        assert_eq!(
            pick_screen(&monitors, Some(fullscreen)),
            monitors[0].screen()
        );
    }
}
