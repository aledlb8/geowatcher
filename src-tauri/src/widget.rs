//! The always-on-top corner widget window.
//!
//! Click-through is not simply a flag: a window that ignores the cursor also
//! stops receiving hover events, so it could never notice the mouse arriving.
//! Instead a poller watches the cursor and lifts click-through only while it is
//! over the widget.

use std::time::Duration;

use tauri::{
    AppHandle, Emitter as _, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::appearance::Appearance;

/// Window label the frontend and commands address it by.
pub const LABEL: &str = "widget";

/// Cursor poll interval. Fast enough to feel immediate, far too cheap to notice.
const HOVER_POLL: Duration = Duration::from_millis(90);

/// Creates the widget window if it is not already open.
pub fn ensure(app: &AppHandle, appearance: &Appearance) -> tauri::Result<()> {
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("GeoWatcher widget")
        .inner_size(f64::from(appearance.width), 120.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(appearance.always_on_top)
        .focused(false)
        .visible(false)
        .build()?;

    Ok(())
}

/// Applies placement and window-level flags. Colours are the page's job.
pub fn apply(app: &AppHandle, appearance: &Appearance) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };

    if !appearance.enabled {
        let _ = window.hide();
        return Ok(());
    }

    window.set_always_on_top(appearance.always_on_top)?;
    window.set_ignore_cursor_events(appearance.click_through)?;
    window.set_size(LogicalSize::new(
        f64::from(appearance.width),
        f64::from(window_height(&window, appearance)),
    ))?;
    reposition(app, appearance)?;
    window.show()?;
    Ok(())
}

/// Current height in logical pixels, falling back to a sensible default.
fn window_height(window: &tauri::WebviewWindow, appearance: &Appearance) -> u32 {
    window
        .inner_size()
        .ok()
        .and_then(|size| {
            window
                .scale_factor()
                .ok()
                .map(|scale| size.to_logical::<f64>(scale).height)
        })
        .map_or_else(
            || appearance.estimated_height(2),
            |height| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let value = height as u32;
                value.max(24)
            },
        )
}

/// Docks the window to its corner of the monitor it currently sits on.
pub fn reposition(app: &AppHandle, appearance: &Appearance) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
        return Ok(());
    };

    let scale = monitor.scale_factor();
    let area = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);
    let size = window.inner_size()?.to_logical::<f64>(scale);

    let x = if appearance.corner.is_right() {
        origin.x + area.width - size.width - f64::from(appearance.margin_x)
    } else {
        origin.x + f64::from(appearance.margin_x)
    };
    let y = if appearance.corner.is_bottom() {
        origin.y + area.height - size.height - f64::from(appearance.margin_y)
    } else {
        origin.y + f64::from(appearance.margin_y)
    };

    window.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}

/// Lifts click-through only while the cursor is over the widget.
///
/// Runs for the life of the app; it is a few float comparisons per tick.
pub fn watch_hover(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut inside = false;
        loop {
            tokio::time::sleep(HOVER_POLL).await;

            let Some(window) = app.get_webview_window(LABEL) else {
                continue;
            };
            if !window.is_visible().unwrap_or(false) {
                continue;
            }

            let now = cursor_is_over(&app, &window).unwrap_or(false);
            if now == inside {
                continue;
            }
            inside = now;

            // Only meaningful for a click-through widget; an interactive one
            // is left alone.
            if wants_click_through(&app) {
                let _ = window.set_ignore_cursor_events(!inside);
            }
            let _ = window.emit_to(LABEL, "widget://hover", inside);
        }
    });
}

fn wants_click_through(app: &AppHandle) -> bool {
    app.try_state::<crate::app::AppState>()
        .is_some_and(|state| state.click_through())
}

fn cursor_is_over(app: &AppHandle, window: &tauri::WebviewWindow) -> tauri::Result<bool> {
    let cursor: PhysicalPosition<f64> = app.cursor_position()?;
    let position = window.outer_position()?;
    let size = window.outer_size()?;

    let left = f64::from(position.x);
    let top = f64::from(position.y);
    let right = left + f64::from(size.width);
    let bottom = top + f64::from(size.height);

    Ok(cursor.x >= left && cursor.x < right && cursor.y >= top && cursor.y < bottom)
}

#[cfg(test)]
mod tests {
    use crate::appearance::{Appearance, Corner};

    /// The placement maths, without a window: x/y for each corner.
    fn place(appearance: &Appearance, area: (f64, f64), size: (f64, f64)) -> (f64, f64) {
        let x = if appearance.corner.is_right() {
            area.0 - size.0 - f64::from(appearance.margin_x)
        } else {
            f64::from(appearance.margin_x)
        };
        let y = if appearance.corner.is_bottom() {
            area.1 - size.1 - f64::from(appearance.margin_y)
        } else {
            f64::from(appearance.margin_y)
        };
        (x, y)
    }

    #[test]
    fn top_left_sits_at_the_margin() {
        let a = Appearance {
            corner: Corner::TopLeft,
            margin_x: 20,
            margin_y: 30,
            ..Appearance::default()
        };
        assert_eq!(place(&a, (1920.0, 1080.0), (300.0, 120.0)), (20.0, 30.0));
    }

    #[test]
    fn bottom_right_accounts_for_the_window_size() {
        let a = Appearance {
            corner: Corner::BottomRight,
            margin_x: 20,
            margin_y: 30,
            ..Appearance::default()
        };
        assert_eq!(
            place(&a, (1920.0, 1080.0), (300.0, 120.0)),
            (1920.0 - 300.0 - 20.0, 1080.0 - 120.0 - 30.0)
        );
    }

    #[test]
    fn a_taller_widget_moves_up_when_docked_to_the_bottom() {
        let a = Appearance {
            corner: Corner::BottomLeft,
            ..Appearance::default()
        };
        let short = place(&a, (1920.0, 1080.0), (300.0, 100.0));
        let tall = place(&a, (1920.0, 1080.0), (300.0, 400.0));
        assert!(tall.1 < short.1);
    }
}
