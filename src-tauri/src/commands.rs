use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};

use crate::db::Db;
use crate::model::{today, TaskKind};
use crate::{dock_state_for, Dock};

pub struct AppState {
    pub db: Arc<Mutex<Db>>,
    pub dock: Dock,
}

fn str_to_kind(s: &str) -> TaskKind {
    match s {
        "daily" => TaskKind::Daily,
        "weekly" => TaskKind::Weekly,
        "interval" => TaskKind::Interval,
        "log" => TaskKind::Log,
        _ => TaskKind::Once,
    }
}

fn refresh(app: &AppHandle, state: &State<AppState>) {
    if let Ok(db) = state.db.lock() {
        let d = today();
        let items = db.view(d).unwrap_or_default();
        let urgent = db.urgent(d, chrono::Local::now().naive_local());
        let _ = app.emit("view-updated", serde_json::json!({ "items": items, "urgent": urgent }));
    }
}

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> serde_json::Value {
    let d = today();
    let (items, urgent, date_str) = match state.db.lock() {
        Ok(db) => (
            db.view(d).unwrap_or_default(),
            db.urgent(d, chrono::Local::now().naive_local()),
            d.format("%Y-%m-%d").to_string(),
        ),
        Err(_) => (vec![], false, String::new()),
    };
    serde_json::json!({ "items": items, "urgent": urgent, "date": date_str })
}

#[tauri::command]
pub fn create_task(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    title: String,
    note: Option<String>,
    pinned: Option<bool>,
    deadline: Option<String>,
    weekly_days: Option<Vec<u8>>,
    interval_days: Option<i64>,
    daily_due_time: Option<String>,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = db
        .create_task(
            str_to_kind(&kind),
            &title,
            note.as_deref(),
            pinned.unwrap_or(false),
            deadline.as_deref(),
            weekly_days.as_deref().unwrap_or(&[]),
            interval_days,
            daily_due_time.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(id)
}

#[tauri::command]
pub fn update_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    title: Option<String>,
    note: Option<String>,
    pinned: Option<bool>,
    deadline: Option<String>,
    weekly_days: Option<Vec<u8>>,
    interval_days: Option<i64>,
    daily_due_time: Option<String>,
    paused: Option<bool>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(v) = title {
        db.update_task_fields(id, &[("title".into(), v)]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = note {
        db.update_task_fields(id, &[("note".into(), v)]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = pinned {
        db.update_task_fields(id, &[("pinned".into(), if v { "1".into() } else { "0".into() })])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = deadline {
        db.update_task_fields(id, &[("deadline".into(), v)]).map_err(|e| e.to_string())?;
    }
    if let Some(days) = weekly_days {
        let s = days.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",");
        db.update_task_fields(id, &[("weekly_days".into(), s)]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = interval_days {
        db.update_task_fields(id, &[("interval_days".into(), v.to_string())])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = daily_due_time {
        db.update_task_fields(id, &[("daily_due_time".into(), v)]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = paused {
        db.update_task_fields(id, &[("paused".into(), if v { "1".into() } else { "0".into() })])
            .map_err(|e| e.to_string())?;
    }
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn delete_task(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_task(id).map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn toggle_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    done: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.toggle_complete(id, today(), done)
        .map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn postpone_task(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.postpone(id, today()).map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn reschedule_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    deadline: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.reschedule(id, &deadline).map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn add_log_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    note: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_log_entry(id, note.as_deref())
        .map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn get_log_entries(
    state: State<'_, AppState>,
    id: i64,
) -> Vec<serde_json::Value> {
    if let Ok(db) = state.db.lock() {
        if let Ok(rows) = db.log_entries(id, 50) {
            return rows
                .into_iter()
                .map(|(at, note)| serde_json::json!({ "at": at, "note": note }))
                .collect();
        }
    }
    vec![]
}

#[tauri::command]
pub fn clear_completed_today(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.clear_completed_today(today())
        .map_err(|e| e.to_string())?;
    drop(db);
    refresh(&app, &state);
    Ok(())
}

// ---------- 停靠 ----------

#[tauri::command]
pub fn dock_to(app: AppHandle, state: State<'_, AppState>, target: String) {
    let s = dock_state_for(&target);
    state.dock.dock_to(&app, s);
}

/// 头部拖动：系统级拖动循环，松手后吸附最近的屏幕左右缘
#[tauri::command]
pub fn begin_header_drag(app: AppHandle, state: State<'_, AppState>) {
    state.dock.begin_drag(&app);
}

#[tauri::command]
pub fn set_dock_side(app: AppHandle, state: State<'_, AppState>, side: String) {
    state.dock.set_edge(&app, crate::dock::Edge::parse(&side));
}

#[tauri::command]
pub fn get_dock_side(state: State<'_, AppState>) -> String {
    state.dock.edge().as_str().to_string()
}

#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Option<String> {
    state.db.lock().ok().and_then(|db| db.get_setting(&key))
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) {
    if let Ok(db) = state.db.lock() {
        db.set_setting(&key, &value);
    }
}

#[tauri::command]
pub fn export_data(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.export_json().map_err(|e| e.to_string())
}
