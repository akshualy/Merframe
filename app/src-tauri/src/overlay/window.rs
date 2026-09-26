use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tracing::{debug, warn};
use wf_log::MonitorRect;

use super::Kind;
use crate::settings::OverlayPlacement;

const MARGIN: f64 = 20.0;

const DEFAULT_SCREEN: Screen = Screen {
    x: 0.0,
    y: 0.0,
    width: 1920.0,
    height: 1080.0,
    scale: 1.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Screen {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) scale: f64,
}

impl Screen {
    fn from_monitor(monitor: &tauri::Monitor) -> Self {
        let position = monitor.position();
        let size = monitor.size();
        Self {
            x: f64::from(position.x),
            y: f64::from(position.y),
            width: f64::from(size.width),
            height: f64::from(size.height),
            scale: monitor.scale_factor(),
        }
    }

    fn from_game(rect: MonitorRect, scale: f64) -> Self {
        Self {
            x: f64::from(rect.left),
            y: f64::from(rect.top),
            width: f64::from(rect.width),
            height: f64::from(rect.height),
            scale,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Bounds {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) x: f64,
    pub(super) y: f64,
}

pub(super) fn logical(bounds: Bounds, scale: f64) -> Bounds {
    Bounds {
        width: bounds.width / scale,
        height: bounds.height / scale,
        x: bounds.x / scale,
        y: bounds.y / scale,
    }
}

pub(super) fn bounds_for(
    kind: Kind,
    placement: OverlayPlacement,
    rows: u8,
    screen: Screen,
) -> Bounds {
    let scale = screen.height / 1080.0;
    let (width, height) = match kind {
        Kind::RelicReward => ((1000.0 * scale).round(), 295.0),
        Kind::RelicRecommendation => (480.0, 60.0 + 30.0 * f64::from(rows)),
        Kind::Riven => (
            700.0,
            (900.0 * scale).min(screen.height - 2.0 * MARGIN).round(),
        ),
    };
    let right = (screen.width - width - MARGIN).round();
    let bottom = (screen.height - height - MARGIN).round();
    let (x, y) = match placement {
        OverlayPlacement::TopLeft => (MARGIN, MARGIN),
        OverlayPlacement::TopRight => (right, MARGIN),
        OverlayPlacement::BottomLeft => (MARGIN, bottom),
        OverlayPlacement::BottomRight => (right, bottom),
        OverlayPlacement::Centre => match kind {
            Kind::RelicReward => (
                (screen.width / 2.0 - width / 2.0 - 15.0).round(),
                (630.0 * scale).round(),
            ),
            Kind::RelicRecommendation | Kind::Riven => (
                (screen.width / 2.0 - width / 2.0).round(),
                (screen.height / 2.0 - height / 2.0).round(),
            ),
        },
    };
    Bounds {
        width,
        height,
        x: x + screen.x,
        y: y + screen.y,
    }
}

pub(super) fn park<R: Runtime>(app: &AppHandle<R>, kind: Kind) {
    let Some(window) = app.get_webview_window(kind.label()) else {
        return;
    };
    if let Err(error) = window.hide() {
        warn!(label = kind.label(), %error, "Overlay hide failed");
    }
}

pub(super) fn retire<R: Runtime>(app: &AppHandle<R>, kind: Kind) {
    let Some(window) = app.get_webview_window(kind.label()) else {
        return;
    };
    if let Err(error) = window.close() {
        warn!(label = kind.label(), %error, "Overlay window close failed");
    }
}

pub(super) fn reveal<R: Runtime>(app: &AppHandle<R>, kind: Kind) {
    let Some(window) = app.get_webview_window(kind.label()) else {
        debug!(label = kind.label(), "No overlay window to show");
        return;
    };
    if let Err(error) = window.show() {
        warn!(label = kind.label(), %error, "Overlay show failed");
        return;
    }
    if let Err(error) = window.set_always_on_top(true) {
        warn!(
            label = kind.label(),
            %error,
            "Overlay always on top failed",
        );
    }
}

pub(super) fn screen_of<R: Runtime>(app: &AppHandle<R>, game: Option<MonitorRect>) -> Screen {
    let primary = app.primary_monitor().ok().flatten();
    let Some(rect) = game else {
        return primary
            .as_ref()
            .map_or(DEFAULT_SCREEN, Screen::from_monitor);
    };
    let centre_x = f64::from(rect.left) + f64::from(rect.width) / 2.0;
    let centre_y = f64::from(rect.top) + f64::from(rect.height) / 2.0;
    let scale = app
        .monitor_from_point(centre_x, centre_y)
        .ok()
        .flatten()
        .or(primary)
        .map_or(1.0, |monitor| monitor.scale_factor());
    Screen::from_game(rect, scale)
}

pub(super) fn open<R: Runtime>(app: &AppHandle<R>, kind: Kind, bounds: Bounds, scale: f64) {
    let built = WebviewWindowBuilder::new(
        app,
        kind.label(),
        WebviewUrl::App(PathBuf::from(kind.route())),
    )
    .title(kind.title())
    .inner_size(bounds.width, bounds.height)
    .position(bounds.x, bounds.y)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .resizable(cfg!(target_os = "linux"))
    .shadow(false)
    .visible(true)
    .build();
    match built {
        Ok(window) => {
            if let Err(error) = window.set_zoom(1.0 / scale) {
                warn!(
                    label = kind.label(),
                    scale,
                    %error,
                    "Webview zoom to game pixels failed",
                );
            }
            if let Err(error) = window.set_ignore_cursor_events(true) {
                warn!(
                    label = kind.label(),
                    %error,
                    "Click through unsupported on this platform"
                );
            }
            if let Err(error) = window.hide() {
                warn!(
                    label = kind.label(),
                    %error,
                    "New overlay window hide failed",
                );
            }
            #[cfg(target_os = "linux")]
            bypass_window_manager(&window, kind);
        }
        Err(error) => warn!(label = kind.label(), %error, "Overlay window build failed"),
    }
}

#[cfg(target_os = "linux")]
fn bypass_window_manager<R: Runtime>(window: &tauri::WebviewWindow<R>, kind: Kind) {
    let owned = window.clone();
    let queued = window.run_on_main_thread(move || {
        use gtk::prelude::WidgetExt;
        if let Some(surface) = owned
            .gtk_window()
            .ok()
            .and_then(|gtk_window| gtk_window.window())
        {
            surface.set_override_redirect(true);
        }
    });
    if let Err(error) = queued {
        warn!(
            label = kind.label(),
            %error,
            "Override redirect queue failed",
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, reason = "bounds are rounded to whole pixels")]
mod tests {
    use super::*;
    use crate::overlay::{Kind, placement_of};
    use crate::settings::{OverlayPlacement, RECOMMENDATION_COUNT_DEFAULT, Settings};

    const FULL_HD: Screen = Screen {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        scale: 1.0,
    };

    #[test]
    fn game_pixels_at_every_scale() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let screen = Screen { scale, ..FULL_HD };
            let physical = bounds_for(
                Kind::RelicRecommendation,
                OverlayPlacement::TopRight,
                RECOMMENDATION_COUNT_DEFAULT,
                screen,
            );
            assert_eq!(
                physical,
                Bounds {
                    width: 480.0,
                    height: 240.0,
                    x: 1420.0,
                    y: 20.0,
                }
            );
            let window = super::logical(physical, scale);
            assert_eq!(window.width * scale, 480.0);
            assert_eq!(window.x * scale, 1420.0);
        }
        assert_eq!(
            super::logical(
                bounds_for(
                    Kind::RelicRecommendation,
                    OverlayPlacement::TopRight,
                    RECOMMENDATION_COUNT_DEFAULT,
                    Screen {
                        scale: 1.25,
                        ..FULL_HD
                    },
                ),
                1.25
            ),
            Bounds {
                width: 384.0,
                height: 192.0,
                x: 1136.0,
                y: 16.0,
            }
        );

        let compositor_upscales_xwayland = Screen {
            x: 0.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
            scale: 1.0,
        };
        let physical = bounds_for(
            Kind::RelicRecommendation,
            OverlayPlacement::TopRight,
            RECOMMENDATION_COUNT_DEFAULT,
            compositor_upscales_xwayland,
        );
        assert_eq!(
            physical,
            Bounds {
                width: 480.0,
                height: 240.0,
                x: 2060.0,
                y: 20.0,
            }
        );
        assert_eq!(super::logical(physical, 1.0), physical);

        let xwayland_carries_the_scaling_factor = Screen {
            x: 0.0,
            y: 0.0,
            width: 3840.0,
            height: 2160.0,
            scale: 2.0,
        };
        let physical = bounds_for(
            Kind::RelicRecommendation,
            OverlayPlacement::TopRight,
            RECOMMENDATION_COUNT_DEFAULT,
            xwayland_carries_the_scaling_factor,
        );
        assert_eq!(
            physical,
            Bounds {
                width: 480.0,
                height: 240.0,
                x: 3340.0,
                y: 20.0,
            }
        );
        assert_eq!(
            super::logical(physical, 2.0),
            Bounds {
                width: 240.0,
                height: 120.0,
                x: 1670.0,
                y: 10.0,
            }
        );
    }

    #[test]
    fn reward_band_per_resolution() {
        let full = bounds_for(
            Kind::RelicReward,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            FULL_HD,
        );
        assert_eq!(
            full,
            Bounds {
                width: 1000.0,
                height: 295.0,
                x: 445.0,
                y: 630.0,
            }
        );

        let tall = bounds_for(
            Kind::RelicReward,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            Screen {
                x: 0.0,
                y: 0.0,
                width: 2560.0,
                height: 1440.0,
                scale: 1.0,
            },
        );
        assert_eq!(tall.width, 1333.0);
        assert_eq!(tall.y, 840.0);
        assert_eq!(tall.x, 599.0);
    }

    #[test]
    fn shipped_placements() {
        let settings = Settings::default();
        assert_eq!(
            placement_of(&settings, Kind::RelicReward),
            OverlayPlacement::Centre
        );
        assert_eq!(
            placement_of(&settings, Kind::RelicRecommendation),
            OverlayPlacement::TopRight
        );
        assert_eq!(
            placement_of(&settings, Kind::Riven),
            OverlayPlacement::TopLeft
        );
        assert_eq!(
            bounds_for(
                Kind::RelicRecommendation,
                OverlayPlacement::TopRight,
                RECOMMENDATION_COUNT_DEFAULT,
                FULL_HD
            ),
            Bounds {
                width: 480.0,
                height: 240.0,
                x: 1420.0,
                y: 20.0,
            }
        );
        assert_eq!(
            bounds_for(
                Kind::Riven,
                OverlayPlacement::TopLeft,
                RECOMMENDATION_COUNT_DEFAULT,
                FULL_HD
            ),
            Bounds {
                width: 700.0,
                height: 900.0,
                x: 20.0,
                y: 20.0,
            }
        );
        assert_eq!(
            bounds_for(
                Kind::Riven,
                OverlayPlacement::TopLeft,
                RECOMMENDATION_COUNT_DEFAULT,
                Screen {
                    x: 0.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 800.0,
                    scale: 1.0,
                }
            )
            .height,
            667.0
        );
    }

    #[test]
    fn placement_corners() {
        let corner = |placement| {
            let bounds = bounds_for(
                Kind::RelicRecommendation,
                placement,
                RECOMMENDATION_COUNT_DEFAULT,
                FULL_HD,
            );
            assert_eq!(bounds.width, 480.0);
            assert_eq!(bounds.height, 240.0);
            (bounds.x, bounds.y)
        };
        assert_eq!(corner(OverlayPlacement::TopLeft), (20.0, 20.0));
        assert_eq!(corner(OverlayPlacement::TopRight), (1420.0, 20.0));
        assert_eq!(corner(OverlayPlacement::BottomLeft), (20.0, 820.0));
        assert_eq!(corner(OverlayPlacement::BottomRight), (1420.0, 820.0));
        assert_eq!(corner(OverlayPlacement::Centre), (720.0, 420.0));

        let riven = |placement| {
            let bounds = bounds_for(
                Kind::Riven,
                placement,
                RECOMMENDATION_COUNT_DEFAULT,
                FULL_HD,
            );
            assert_eq!(bounds.width, 700.0);
            assert_eq!(bounds.height, 900.0);
            (bounds.x, bounds.y)
        };
        assert_eq!(riven(OverlayPlacement::TopLeft), (20.0, 20.0));
        assert_eq!(riven(OverlayPlacement::TopRight), (1200.0, 20.0));
        assert_eq!(riven(OverlayPlacement::BottomLeft), (20.0, 160.0));
        assert_eq!(riven(OverlayPlacement::BottomRight), (1200.0, 160.0));
        assert_eq!(riven(OverlayPlacement::Centre), (610.0, 90.0));

        let reward = |placement| {
            let bounds = bounds_for(
                Kind::RelicReward,
                placement,
                RECOMMENDATION_COUNT_DEFAULT,
                FULL_HD,
            );
            assert_eq!(bounds.width, 1000.0);
            assert_eq!(bounds.height, 295.0);
            (bounds.x, bounds.y)
        };
        assert_eq!(reward(OverlayPlacement::TopLeft), (20.0, 20.0));
        assert_eq!(reward(OverlayPlacement::TopRight), (900.0, 20.0));
        assert_eq!(reward(OverlayPlacement::BottomLeft), (20.0, 765.0));
        assert_eq!(reward(OverlayPlacement::BottomRight), (900.0, 765.0));
        assert_eq!(reward(OverlayPlacement::Centre), (445.0, 630.0));
    }

    #[test]
    fn larger_screen() {
        let screen = Screen {
            x: 0.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
            scale: 1.0,
        };
        assert_eq!(
            bounds_for(
                Kind::RelicRecommendation,
                OverlayPlacement::BottomRight,
                RECOMMENDATION_COUNT_DEFAULT,
                screen
            ),
            Bounds {
                width: 480.0,
                height: 240.0,
                x: 2060.0,
                y: 1180.0,
            }
        );
        assert_eq!(
            bounds_for(
                Kind::Riven,
                OverlayPlacement::TopRight,
                RECOMMENDATION_COUNT_DEFAULT,
                screen
            ),
            Bounds {
                width: 700.0,
                height: 1200.0,
                x: 1840.0,
                y: 20.0,
            }
        );
    }

    #[test]
    fn game_monitor_origin_moves_the_window() {
        let second_monitor = Screen {
            x: 1920.0,
            y: -360.0,
            width: 2560.0,
            height: 1440.0,
            scale: 1.0,
        };
        let top_left = bounds_for(
            Kind::Riven,
            OverlayPlacement::TopLeft,
            RECOMMENDATION_COUNT_DEFAULT,
            second_monitor,
        );
        assert_eq!((top_left.x, top_left.y), (1940.0, -340.0));
        let bottom_right = bounds_for(
            Kind::Riven,
            OverlayPlacement::BottomRight,
            RECOMMENDATION_COUNT_DEFAULT,
            second_monitor,
        );
        assert_eq!((bottom_right.width, bottom_right.height), (700.0, 1200.0));
        assert_eq!((bottom_right.x, bottom_right.y), (3760.0, -140.0));
        let centre = bounds_for(
            Kind::RelicReward,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            second_monitor,
        );
        assert_eq!((centre.x, centre.y), (2519.0, 480.0));
        let scaled = super::logical(top_left, 2.0);
        assert_eq!((scaled.x, scaled.y), (970.0, -170.0));
    }

    #[test]
    fn game_rectangle_becomes_the_screen() {
        let rect = MonitorRect {
            left: 1920,
            top: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(
            Screen::from_game(rect, 1.25),
            Screen {
                x: 1920.0,
                y: 0.0,
                scale: 1.25,
                ..FULL_HD
            }
        );
    }
}
