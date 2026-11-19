use std::path::PathBuf;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tracing::{debug, warn};

use super::game_screen::ScaledBounds;
use super::{Kind, placement_of};
use crate::settings::{OverlayPlacement, Settings, TextSize};

const MARGIN: f64 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Bounds {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) x: f64,
    pub(super) y: f64,
}

impl Bounds {
    pub(super) fn area(self) -> f64 {
        self.width * self.height
    }

    pub(super) fn overlap(self, other: Self) -> Option<Self> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > left && bottom > top).then_some(Self {
            width: right - left,
            height: bottom - top,
            x: left,
            y: top,
        })
    }
}

pub(super) fn window_bounds(screen: ScaledBounds, kind: Kind, settings: &Settings) -> ScaledBounds {
    let bounds = bounds_for(
        kind,
        placement_of(settings, kind),
        settings.overlays.overlay_recommendation_count,
        settings.text_size,
        screen.bounds,
    );
    ScaledBounds {
        bounds,
        scale: screen.scale,
    }
}

pub(super) fn bounds_for(
    kind: Kind,
    placement: OverlayPlacement,
    rows: u8,
    text_size: TextSize,
    screen: Bounds,
) -> Bounds {
    let scale = screen.height / 1080.0;
    let text = text_size.factor();
    let (width, height) = match kind {
        Kind::RelicReward => (1000.0 * scale * text, 295.0 * text),
        Kind::RelicRecommendation => (480.0 * text, (60.0 + 30.0 * f64::from(rows)) * text),
        Kind::Riven => (700.0 * text, 900.0 * scale * text),
        Kind::Ducatering => (420.0 * text, 460.0 * text),
        Kind::Notification => (380.0 * text, 100.0 * text),
        Kind::ArbitrationLive => (420.0 * text, 190.0 * text),
    };
    let width = width.round();
    let height = height.min(screen.height - 2.0 * MARGIN).round();
    let middle = (screen.width / 2.0 - width / 2.0).round();
    let right = (screen.width - width - MARGIN).round();
    let bottom = (screen.height - height - MARGIN).round();
    let (x, y) = match placement {
        OverlayPlacement::TopLeft => (MARGIN, MARGIN),
        OverlayPlacement::TopCentre => (middle, MARGIN),
        OverlayPlacement::TopRight => (right, MARGIN),
        OverlayPlacement::BottomLeft => (MARGIN, bottom),
        OverlayPlacement::BottomCentre => (middle, bottom),
        OverlayPlacement::BottomRight => (right, bottom),
        OverlayPlacement::Centre => match kind {
            Kind::RelicReward => (middle - 15.0, (630.0 * scale).round()),
            Kind::RelicRecommendation
            | Kind::Riven
            | Kind::Ducatering
            | Kind::Notification
            | Kind::ArbitrationLive => (middle, (screen.height / 2.0 - height / 2.0).round()),
        },
    };
    Bounds {
        width,
        height,
        x: x + screen.x,
        y: y + screen.y,
    }
}

pub(super) fn place<R: Runtime>(window: &WebviewWindow<R>, target: ScaledBounds) {
    let Bounds {
        width,
        height,
        x,
        y,
    } = target.bounds;
    let position = PhysicalPosition::new(x, y);
    let placed = window
        .set_position(position)
        .and_then(|()| window.set_size(PhysicalSize::new(width, height)))
        .and_then(|()| window.set_position(position))
        .and_then(|()| window.set_zoom(1.0 / target.scale));
    if let Err(error) = placed {
        warn!(label = window.label(), %error, "Overlay window resize failed");
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

pub(super) fn open<R: Runtime>(app: &AppHandle<R>, kind: Kind, target: ScaledBounds) {
    let built = WebviewWindowBuilder::new(
        app,
        kind.label(),
        WebviewUrl::App(PathBuf::from(kind.route())),
    )
    .title(kind.title())
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .resizable(cfg!(target_os = "linux"))
    .shadow(false)
    .visible(false)
    .build();
    match built {
        Ok(window) => {
            place(&window, target);

            #[cfg(target_os = "linux")]
            bypass_window_manager(&window, kind);
            #[cfg(not(target_os = "linux"))]
            click_through(&window, kind);
        }
        Err(error) => warn!(label = kind.label(), %error, "Overlay window build failed"),
    }
}

fn click_through<R: Runtime>(window: &WebviewWindow<R>, kind: Kind) {
    if let Err(error) = window.set_ignore_cursor_events(true) {
        warn!(
            label = kind.label(),
            %error,
            "Overlay click through failed"
        );
    }
}

#[cfg(target_os = "linux")]
fn bypass_window_manager<R: Runtime>(window: &WebviewWindow<R>, kind: Kind) {
    let owned = window.clone();
    let queued = window.run_on_main_thread(move || {
        use gtk::prelude::WidgetExt;
        if let Ok(gtk_window) = owned.gtk_window() {
            gtk_window.realize();
            if let Some(surface) = gtk_window.window() {
                surface.set_override_redirect(true);
            }
        }

        click_through(&owned, kind);
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
    use crate::settings::RECOMMENDATION_COUNT_DEFAULT;

    const FULL_HD: Bounds = Bounds {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };

    #[test]
    fn text_size_grows_the_window_in_place() {
        let medium = bounds_for(
            Kind::Ducatering,
            OverlayPlacement::BottomRight,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Medium,
            FULL_HD,
        );
        assert_eq!(
            medium,
            Bounds {
                width: 473.0,
                height: 518.0,
                x: 1427.0,
                y: 542.0,
            }
        );

        let large = bounds_for(
            Kind::Riven,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Large,
            FULL_HD,
        );
        assert_eq!(large.width, 875.0);
        assert_eq!(large.height, FULL_HD.height - 2.0 * MARGIN);
    }

    #[test]
    fn reward_band_per_resolution() {
        let full = bounds_for(
            Kind::RelicReward,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Small,
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
            TextSize::Small,
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 2560.0,
                height: 1440.0,
            },
        );
        assert_eq!(tall.width, 1333.0);
        assert_eq!(tall.y, 840.0);
        assert_eq!(tall.x, 599.0);
    }

    #[test]
    fn notification_positions() {
        let at = |position: &str| {
            let settings: Settings =
                serde_json::from_str(&format!(r#"{{"toasts_in_game_position":"{position}"}}"#))
                    .unwrap();
            let bounds = bounds_for(
                Kind::Notification,
                placement_of(&settings, Kind::Notification),
                RECOMMENDATION_COUNT_DEFAULT,
                TextSize::Small,
                FULL_HD,
            );
            assert_eq!((bounds.width, bounds.height), (380.0, 100.0));
            (bounds.x, bounds.y)
        };
        assert_eq!(at("top_left"), (20.0, 20.0));
        assert_eq!(at("top_centre"), (770.0, 20.0));
        assert_eq!(at("top_right"), (1520.0, 20.0));
        assert_eq!(at("bottom_left"), (20.0, 960.0));
        assert_eq!(at("bottom_centre"), (770.0, 960.0));
        assert_eq!(at("bottom_right"), (1520.0, 960.0));
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
                TextSize::Small,
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
                TextSize::Small,
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
                TextSize::Small,
                Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 800.0,
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
                TextSize::Small,
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
                TextSize::Small,
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
                TextSize::Small,
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
        let screen = Bounds {
            x: 0.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
        };
        assert_eq!(
            bounds_for(
                Kind::RelicRecommendation,
                OverlayPlacement::BottomRight,
                RECOMMENDATION_COUNT_DEFAULT,
                TextSize::Small,
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
                TextSize::Small,
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
        let second_monitor = Bounds {
            x: 1920.0,
            y: -360.0,
            width: 2560.0,
            height: 1440.0,
        };
        let top_left = bounds_for(
            Kind::Riven,
            OverlayPlacement::TopLeft,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Small,
            second_monitor,
        );
        assert_eq!((top_left.x, top_left.y), (1940.0, -340.0));
        let bottom_right = bounds_for(
            Kind::Riven,
            OverlayPlacement::BottomRight,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Small,
            second_monitor,
        );
        assert_eq!((bottom_right.width, bottom_right.height), (700.0, 1200.0));
        assert_eq!((bottom_right.x, bottom_right.y), (3760.0, -140.0));
        let centre = bounds_for(
            Kind::RelicReward,
            OverlayPlacement::Centre,
            RECOMMENDATION_COUNT_DEFAULT,
            TextSize::Small,
            second_monitor,
        );
        assert_eq!((centre.x, centre.y), (2519.0, 480.0));
    }
}
