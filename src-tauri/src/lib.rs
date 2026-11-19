pub mod commands;
pub mod db;
pub mod dock;
pub mod model;
pub mod remind;

use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use db::Db;

pub use dock::{Dock, DockState};

pub fn dock_state_for(s: &str) -> DockState {
    match s {
        "panel" => DockState::Panel,
        "handle" => DockState::Handle,
        _ => DockState::Hidden,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let db = Db::open(&app.path().app_data_dir()?)?;
            let db = Arc::new(Mutex::new(db));

            let dock = Dock::new(db.clone());
            app.manage(commands::AppState {
                db: db.clone(),
                dock: dock.clone(),
            });

            // 主窗口：按持久化配置停靠（显示器/侧/纵位），就位后再显示
            if let Some(win) = app.get_webview_window("main") {
                dock.init(&win);
                let _ = win.show();
                // show 之后剥系统标题栏（tao 在 show 时会重应用缓存的窗口样式，
                // 提前剥会被覆盖回去）
                dock.strip_native_frame(&win);

                // 首次出现：微光 → 把手，让用户注意到边缘有东西
                let app2 = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    let state = app2.state::<commands::AppState>();
                    state.dock.dock_to(&app2, DockState::Handle);
                });
            }

            // 托盘
            let show = MenuItem::with_id(app, "show", "展开面板", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "收起隐藏", true, None::<&str>)?;
            let dock_l = MenuItem::with_id(app, "dock-left", "停靠到屏幕左缘", true, None::<&str>)?;
            let dock_r = MenuItem::with_id(app, "dock-right", "停靠到屏幕右缘", true, None::<&str>)?;
            let dock_t = MenuItem::with_id(app, "dock-top", "停靠到屏幕上缘", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &dock_l, &dock_r, &dock_t, &quit])?;
            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let state = app.state::<commands::AppState>();
                    match event.id.as_ref() {
                        "show" => state.dock.dock_to(app, DockState::Panel),
                        "hide" => state.dock.dock_to(app, DockState::Hidden),
                        "dock-left" => state.dock.set_edge(app, crate::dock::Edge::Left),
                        "dock-right" => state.dock.set_edge(app, crate::dock::Edge::Right),
                        "dock-top" => state.dock.set_edge(app, crate::dock::Edge::Top),
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle().clone();
                        let state = app.state::<commands::AppState>();
                        state.dock.dock_to(&app, DockState::Panel);
                    }
                })
                .build(app)?;

            // 到期提醒线程
            remind::spawn(app.handle().clone(), db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::toggle_complete,
            commands::postpone_task,
            commands::reschedule_task,
            commands::add_log_entry,
            commands::get_log_entries,
            commands::clear_completed_today,
            commands::dock_to,
            commands::begin_header_drag,
            commands::set_dock_side,
            commands::get_dock_side,
            commands::get_setting,
            commands::set_setting,
            commands::export_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
