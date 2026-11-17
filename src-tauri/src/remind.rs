//! 到期提醒：每 30s 扫描一次，未来 N 分钟内到期且未完成的任务弹系统通知；
//! 紧急状态（已过期或 1h 内到期）通过事件驱动把手呼吸灯。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tauri::Emitter;
use tauri_plugin_notification::NotificationExt;

use crate::db::Db;
use crate::model::today;

pub fn spawn(app: tauri::AppHandle, db: Arc<Mutex<Db>>) {
    std::thread::spawn(move || {
        let mut notified: HashSet<(i64, String)> = HashSet::new();
        let mut last_urgent: Option<bool> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let Ok(db) = db.lock() else { return };
            let d = today();
            let now = chrono::Local::now().naive_local();

            // 到期通知（30 分钟提前量；当日同任务只提醒一次）
            let day_key = d.format("%Y-%m-%d").to_string();
            for (id, title, due) in db.upcoming(d, 30, now) {
                let key = (id, day_key.clone());
                if notified.contains(&key) {
                    continue;
                }
                notified.insert(key);
                let hm = due.format("%H:%M").to_string();
                let _ = app
                    .notification()
                    .builder()
                    .title("边芽 · 任务即将到期")
                    .body(format!("「{title}」将于 {hm} 到期"))
                    .show();
            }

            // 紧急态（呼吸灯）
            let urgent = db.urgent(d, now);
            if last_urgent != Some(urgent) {
                last_urgent = Some(urgent);
                let _ = app.emit("urgent-changed", urgent);
            }
        }
    });
}
