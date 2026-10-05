//! Interactive Apps & Scratchpads picker panel.

use crate::apps::model::{AppConfig, AppLaunchMode, TrackedWindow};
use crate::config::Config;
use crate::ui::picker::PickerItem;
use crate::ui::preview::Preview;

/// Builds picker items for applications, FloaX scratchpad, tracked windows, and quick actions.
pub fn build_picker_items(
    apps: &[AppConfig],
    floax_running: bool,
    tracked_windows: &[TrackedWindow],
    current_window_tracked: bool,
) -> Vec<PickerItem> {
    let mut items = Vec::with_capacity(apps.len() + 1 + tracked_windows.len() + 2);

    // 1. Configured apps
    for app in apps {
        let label = match &app.key {
            Some(key) => format!("{} [{key}]", app.title),
            None => app.title.clone(),
        };
        let mode_desc = match &app.mode {
            AppLaunchMode::Popup { width, height } => format!("popup ({width}x{height})"),
            AppLaunchMode::Window => "window".to_string(),
        };
        let detail = format!("{} · {}", app.command, mode_desc);
        let preview = Preview::Text(format!(
            "Application: {}\nCommand: {}\nMode: {}\nKey hint: {}\nFocus existing: {}",
            app.title,
            app.command,
            mode_desc,
            app.key.as_deref().unwrap_or("none"),
            app.focus_existing
        ));

        items.push(PickerItem {
            id: format!("app:{}", app.name),
            label,
            detail,
            preview,
        });
    }

    // 2. FloaX scratchpad
    let status_str = if floax_running { "RUNNING" } else { "IDLE" };
    let floax_label = format!("FloaX Scratchpad [{status_str}]");
    let floax_detail = format!("persistent scratchpad terminal · {status_str}");
    let floax_preview = Preview::Text(format!(
        "FloaX Scratchpad\nState: {status_str}\nSession: floax\nToggle behavior: popup overlay"
    ));
    items.push(PickerItem {
        id: "floax".to_string(),
        label: floax_label,
        detail: floax_detail,
        preview: floax_preview,
    });

    // 3. Tracked windows
    for win in tracked_windows {
        let win_label = format!("window · {} ({})", win.name, win.id);
        let win_detail = format!("id: {}, session: {}", win.id, win.session_id);
        let win_preview = Preview::Text(format!(
            "Tracked Window: {}\nID: {}\nSession: {}\nActive: {}",
            win.name, win.id, win.session_id, win.active
        ));
        items.push(PickerItem {
            id: format!("window:{}", win.id),
            label: win_label,
            detail: win_detail,
            preview: win_preview,
        });
    }

    // 4. Action: track or untrack current window
    if current_window_tracked {
        items.push(PickerItem {
            id: "action:untrack_current".to_string(),
            label: "[-] Untrack current window".to_string(),
            detail: "unmark current tmux window from tracking".to_string(),
            preview: Preview::Text("Untrack current window from quick-toggle list".to_string()),
        });
    } else {
        items.push(PickerItem {
            id: "action:track_current".to_string(),
            label: "[+] Track current window".to_string(),
            detail: "mark current tmux window as tracked".to_string(),
            preview: Preview::Text("Track current window for quick-toggle navigation".to_string()),
        });
    }

    // 5. Action: toggle tracked window
    items.push(PickerItem {
        id: "action:toggle_track".to_string(),
        label: "Toggle tracked window".to_string(),
        detail: "quick toggle between tracked windows".to_string(),
        preview: Preview::Text(
            "Toggle to last tracked window or back to previous window".to_string(),
        ),
    });

    items
}

/// Executes an action or launches an app based on selected picker item ID.
pub fn execute_selected_item(
    socket: Option<&str>,
    selected_id: &str,
    apps: &[AppConfig],
) -> Result<(), String> {
    if let Some(app_name) = selected_id.strip_prefix("app:") {
        if let Some(app) = apps.iter().find(|a| a.name == app_name) {
            crate::apps::runner::run_app(socket, app, None, false)
        } else {
            Err(format!("Unknown application: {app_name}"))
        }
    } else if selected_id == "floax" {
        crate::apps::floax::toggle_floax(socket, &crate::apps::floax::FloaxConfig::default(), None)
    } else if let Some(win_id) = selected_id.strip_prefix("window:") {
        crate::apps::tracking::toggle_tracked_window(socket, Some(win_id))
    } else {
        match selected_id {
            "action:track_current" => crate::apps::tracking::track_window(socket, None).map(|_| ()),
            "action:untrack_current" => {
                crate::apps::tracking::untrack_window(socket, None).map(|_| ())
            }
            "action:toggle_track" => crate::apps::tracking::toggle_tracked_window(socket, None),
            _ => Err(format!("Unknown selection: {selected_id}")),
        }
    }
}

/// Runs the interactive Apps and Scratchpads picker UI.
pub fn run_apps_panel(socket: Option<&str>, config: Option<&Config>) -> Result<(), String> {
    let loaded_config;
    let cfg = match config {
        Some(cfg) => cfg,
        None => {
            loaded_config = crate::config::loader::load_config(None);
            &loaded_config
        }
    };

    let floax_running = crate::apps::floax::has_floax_session(
        socket,
        &crate::apps::floax::FloaxConfig::default().session_name,
    );
    let tracked_windows = crate::apps::tracking::list_tracked_windows(socket).unwrap_or_default();
    let current_window_tracked =
        crate::apps::tracking::is_window_tracked(socket, None).unwrap_or(false);

    let _theme = crate::ui::theme::Theme::from_config(cfg);
    let items = build_picker_items(
        &cfg.apps,
        floax_running,
        &tracked_windows,
        current_window_tracked,
    );

    if let Some(index) = crate::ui::picker::run_picker("Apps & Scratchpads", &items, socket)?
        && let Some(item) = items.get(index)
    {
        execute_selected_item(socket, &item.id, &cfg.apps)?;
    }

    Ok(())
}
