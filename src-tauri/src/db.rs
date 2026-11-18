use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Row};

use crate::model::{
    build_view, effective_due, fmt_datetime, occurs_today, parse_datetime, TaskKind,
    TaskRow, ViewItem,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    kind           TEXT NOT NULL,             -- once/daily/weekly/interval/log
    title          TEXT NOT NULL,
    note           TEXT,
    pinned         INTEGER NOT NULL DEFAULT 0,
    sort_order     REAL NOT NULL DEFAULT 0,
    deadline       TEXT,                      -- once: "2026-11-20" 或 "2026-11-20T18:00"
    weekly_days    TEXT,                      -- weekly: "1,3,5"
    interval_days  INTEGER,
    daily_due_time TEXT,                      -- 重复任务每日截止 "22:00"
    paused         INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL,
    archived_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_tasks_kind ON tasks(kind);

CREATE TABLE IF NOT EXISTS completions (
    task_id    INTEGER NOT NULL,
    date       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, date)
);

CREATE TABLE IF NOT EXISTS postpones (
    task_id    INTEGER NOT NULL,
    date       TEXT NOT NULL,
    count      INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (task_id, date)
);

CREATE TABLE IF NOT EXISTS log_entries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id    INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    note       TEXT
);
CREATE INDEX IF NOT EXISTS idx_log_task ON log_entries(task_id);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

fn kind_to_str(k: TaskKind) -> &'static str {
    match k {
        TaskKind::Once => "once",
        TaskKind::Daily => "daily",
        TaskKind::Weekly => "weekly",
        TaskKind::Interval => "interval",
        TaskKind::Log => "log",
    }
}

fn kind_from_str(s: &str) -> TaskKind {
    match s {
        "daily" => TaskKind::Daily,
        "weekly" => TaskKind::Weekly,
        "interval" => TaskKind::Interval,
        "log" => TaskKind::Log,
        _ => TaskKind::Once,
    }
}

fn parse_time(s: &str) -> Option<chrono::NaiveTime> {
    chrono::NaiveTime::parse_from_str(s, "%H:%M").ok()
}

fn row_to_task(r: &Row) -> rusqlite::Result<TaskRow> {
    let kind_s: String = r.get("kind")?;
    let weekly_days: Option<String> = r.get("weekly_days")?;
    let deadline: Option<String> = r.get("deadline")?;
    let due_time: Option<String> = r.get("daily_due_time")?;
    let created_at: Option<String> = r.get("created_at")?;
    Ok(TaskRow {
        id: r.get("id")?,
        kind: kind_from_str(&kind_s),
        title: r.get("title")?,
        note: r.get("note")?,
        pinned: r.get::<_, i64>("pinned")? != 0,
        sort_order: r.get("sort_order")?,
        deadline: deadline.as_deref().and_then(parse_datetime),
        weekly_days: weekly_days
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|p| p.trim().parse::<u8>().ok())
                    .collect()
            })
            .unwrap_or_default(),
        interval_days: r.get("interval_days")?,
        daily_due_time: due_time.as_deref().and_then(parse_time),
        paused: r.get::<_, i64>("paused")? != 0,
        created_at: created_at
            .as_deref()
            .and_then(parse_datetime)
            .unwrap_or_else(|| Local::now().naive_local()),
        archived_at: r
            .get::<_, Option<String>>("archived_at")?
            .as_deref()
            .and_then(parse_datetime),
    })
}

pub struct Db(pub Connection);

impl Db {
    pub fn open(dir: &Path) -> rusqlite::Result<Self> {
        std::fs::create_dir_all(dir).ok();
        let conn = Connection::open(dir.join("edge.db"))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Db(conn))
    }

    fn all_tasks(&self) -> rusqlite::Result<Vec<TaskRow>> {
        let mut st = self.0.prepare("SELECT * FROM tasks")?;
        let rows = st.query_map([], row_to_task)?;
        rows.collect()
    }

    /// 有任意完成记录的一次性任务 id 集（一次性任务 = 二态，跨天不复活）
    fn once_done_ids(&self) -> HashSet<i64> {
        let mut st = match self
            .0
            .prepare("SELECT DISTINCT c.task_id FROM completions c JOIN tasks t ON t.id = c.task_id WHERE t.kind = 'once'")
        {
            Ok(s) => s,
            Err(_) => return HashSet::new(),
        };
        let x = st
            .query_map([], |r| r.get::<_, i64>(0))
            .and_then(|rows| rows.collect::<rusqlite::Result<_>>())
            .unwrap_or_default();
        x
    }

    fn task(&self, id: i64) -> rusqlite::Result<Option<TaskRow>> {
        let mut st = self.0.prepare("SELECT * FROM tasks WHERE id = ?1")?;
        let mut rows = st.query_map(params![id], row_to_task)?;
        Ok(rows.next().transpose()?)
    }

    // ---------- 视图 ----------

    pub fn view(&self, d: NaiveDate) -> rusqlite::Result<Vec<ViewItem>> {
        let ds = d.format("%Y-%m-%d").to_string();

        let completed: HashSet<i64> = {
            let mut st = self
                .0
                .prepare("SELECT task_id FROM completions WHERE date = ?1")?;
            let x = st
                .query_map(params![ds], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<_>>()?;
            x
        };

        let postponed: HashSet<i64> = {
            let mut st = self.0.prepare("SELECT task_id FROM postpones WHERE date = ?1")?;
            let x = st
                .query_map(params![ds], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<_>>()?;
            x
        };

        let log_today: HashMap<i64, i64> = {
            let mut st = self.0.prepare(
                "SELECT task_id, COUNT(*) FROM log_entries WHERE created_at >= ?1 AND created_at < ?2 GROUP BY task_id",
            )?;
            let next = (d + Duration::days(1)).format("%Y-%m-%d").to_string();
            let x = st
                .query_map(params![ds, next], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
                })?
                .collect::<rusqlite::Result<_>>()?;
            x
        };

        let log_total: HashMap<i64, i64> = {
            let mut st = self
                .0
                .prepare("SELECT task_id, COUNT(*) FROM log_entries GROUP BY task_id")?;
            let x = st
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            x
        };

        let postpone_counts: HashMap<i64, i64> = {
            let mut st = self
                .0
                .prepare("SELECT task_id, SUM(count) FROM postpones GROUP BY task_id")?;
            let x = st
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            x
        };

        let tasks = self.all_tasks()?;
        Ok(build_view(
            &tasks,
            d,
            &completed,
            &self.once_done_ids(),
            &postponed,
            &log_today,
            &log_total,
            &postpone_counts,
        ))
    }

    // ---------- 任务 CRUD ----------

    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
        &self,
        kind: TaskKind,
        title: &str,
        note: Option<&str>,
        pinned: bool,
        deadline: Option<&str>,
        weekly_days: &[u8],
        interval_days: Option<i64>,
        daily_due_time: Option<&str>,
    ) -> rusqlite::Result<i64> {
        let now = Local::now().format("%Y-%m-%dT%H:%M").to_string();
        let max_order: f64 = self
            .0
            .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM tasks", [], |r| {
                r.get(0)
            })
            .unwrap_or(0.0);
        self.0.execute(
            "INSERT INTO tasks (kind, title, note, pinned, sort_order, deadline, weekly_days, interval_days, daily_due_time, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                kind_to_str(kind),
                title,
                note,
                pinned as i64,
                max_order + 1.0,
                deadline,
                weekly_days
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                interval_days,
                daily_due_time,
                now
            ],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn update_task_fields(&self, id: i64, sets: &[(String, String)]) -> rusqlite::Result<()> {
        if sets.is_empty() {
            return Ok(());
        }
        let assignments: Vec<String> = sets.iter().map(|(k, _)| format!("{k} = ?")).collect();
        // id 直接内插（i64 无注入风险），占位符数量 = 字段数
        let sql = format!("UPDATE tasks SET {} WHERE id = {}", assignments.join(", "), id);
        let mut st = self.0.prepare(&sql)?;
        let mut idx = 1;
        for (_, v) in sets {
            st.raw_bind_parameter(idx, v)?;
            idx += 1;
        }
        st.raw_execute()?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        self.0
            .execute("DELETE FROM completions WHERE task_id = ?1", params![id])?;
        self.0
            .execute("DELETE FROM postpones WHERE task_id = ?1", params![id])?;
        self.0
            .execute("DELETE FROM log_entries WHERE task_id = ?1", params![id])?;
        Ok(())
    }

    // ---------- 状态操作 ----------

    pub fn toggle_complete(&self, id: i64, d: NaiveDate, done: bool) -> rusqlite::Result<()> {
        let ds = d.format("%Y-%m-%d").to_string();
        if done {
            let now = Local::now().format("%Y-%m-%dT%H:%M").to_string();
            self.0.execute(
                "INSERT OR REPLACE INTO completions (task_id, date, created_at) VALUES (?1,?2,?3)",
                params![id, ds, now],
            )?;
        } else {
            let kind: Option<String> = self
                .0
                .query_row("SELECT kind FROM tasks WHERE id = ?1", params![id], |r| {
                    r.get(0)
                })
                .ok();
            if kind.as_deref() == Some("once") {
                // 一次性任务是二态：撤销 = 删除全部完成记录（否则跨天会误判已完成）
                self.0
                    .execute("DELETE FROM completions WHERE task_id = ?1", params![id])?;
            } else {
                self.0.execute(
                    "DELETE FROM completions WHERE task_id = ?1 AND date = ?2",
                    params![id, ds],
                )?;
            }
        }
        Ok(())
    }

    /// 延期：一次性顺延截止到明天（保留原时间）；重复任务记今日延期
    pub fn postpone(&self, id: i64, d: NaiveDate) -> rusqlite::Result<()> {
        let ds = d.format("%Y-%m-%d").to_string();
        let now = Local::now().format("%Y-%m-%dT%H:%M").to_string();
        if let Some(t) = self.task(id)? {
            match t.kind {
                TaskKind::Once => {
                    let base = t.deadline.map(|dt| dt.date()).unwrap_or(d);
                    let time = t.deadline.map(|dt| dt.time());
                    // 已过期的任务延期时直接跳到明天，而不是从过去的日期 +1
                    let new_dl = std::cmp::max(base + Duration::days(1), d + Duration::days(1));
                    let dl_str = match time {
                        Some(tm) => new_dl.format("%Y-%m-%d").to_string() + &tm.format("T%H:%M").to_string(),
                        None => new_dl.format("%Y-%m-%d").to_string(),
                    };
                    self.0.execute(
                        "UPDATE tasks SET deadline = ?2 WHERE id = ?1",
                        params![id, dl_str],
                    )?;
                    // 累计延期次数
                    self.0.execute(
                        "INSERT INTO postpones (task_id, date, count) VALUES (?1, ?2, 1)
                         ON CONFLICT(task_id, date) DO UPDATE SET count = count + 1",
                        params![id, now[..10].to_string()],
                    )?;
                }
                _ => {
                    self.0.execute(
                        "INSERT INTO postpones (task_id, date, count) VALUES (?1, ?2, 1)
                         ON CONFLICT(task_id, date) DO UPDATE SET count = count + 1",
                        params![id, ds],
                    )?;
                }
            }
        }
        Ok(())
    }

    /// 改期（一次性任务直接设截止）
    pub fn reschedule(&self, id: i64, deadline: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE tasks SET deadline = ?2 WHERE id = ?1",
            params![id, deadline],
        )?;
        Ok(())
    }

    pub fn add_log_entry(&self, id: i64, note: Option<&str>) -> rusqlite::Result<i64> {
        let now = Local::now().format("%Y-%m-%dT%H:%M").to_string();
        self.0.execute(
            "INSERT INTO log_entries (task_id, created_at, note) VALUES (?1,?2,?3)",
            params![id, now, note],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn log_entries(&self, id: i64, limit: i64) -> rusqlite::Result<Vec<(String, Option<String>)>> {
        let mut st = self.0.prepare(
            "SELECT created_at, note FROM log_entries WHERE task_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![id, limit], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        rows.collect()
    }

    pub fn clear_completed_today(&self, d: NaiveDate) -> rusqlite::Result<()> {
        let ds = d.format("%Y-%m-%d").to_string();
        // 归档所有有完成记录的一次性任务（不限当日——一次性任务是二态）
        self.0.execute(
            "UPDATE tasks SET archived_at = ?1 WHERE kind = 'once' AND id IN (SELECT task_id FROM completions)",
            params![Local::now().format("%Y-%m-%dT%H:%M").to_string()],
        )?;
        self.0.execute(
            "DELETE FROM completions WHERE task_id IN (SELECT id FROM tasks WHERE kind = 'once')",
            [],
        )?;
        // 重复任务仅清掉当日完成记录
        self.0.execute("DELETE FROM completions WHERE date = ?1", params![ds])?;
        Ok(())
    }

    // ---------- 提醒扫描 ----------

    /// 返回 (task_id, title, due) — 未来 within 内到期、且尚未到期的任务（今日视图内）
    pub fn upcoming(&self, d: NaiveDate, within_min: i64, now: NaiveDateTime) -> Vec<(i64, String, NaiveDateTime)> {
        let Ok(tasks) = self.all_tasks() else { return vec![] };
        let ds = d.format("%Y-%m-%d").to_string();
        let mut done: HashSet<i64> = self
            .0
            .prepare("SELECT task_id FROM completions WHERE date = ?1")
            .and_then(|mut st| {
                st.query_map(params![ds], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<_>>()
            })
            .unwrap_or_default();
        done.extend(self.once_done_ids());
        let mut out = vec![];
        for t in tasks {
            if t.archived_at.is_some() || done.contains(&t.id) {
                continue;
            }
            if let Some(due) = effective_due(&t, d) {
                let delta = (due - now).num_minutes();
                if delta >= 0 && delta <= within_min {
                    out.push((t.id, t.title.clone(), due));
                }
            }
        }
        out
    }

    /// 是否存在已过期或 1 小时内到期的任务（把手呼吸灯）
    pub fn urgent(&self, d: NaiveDate, now: NaiveDateTime) -> bool {
        let Ok(tasks) = self.all_tasks() else { return false };
        let ds = d.format("%Y-%m-%d").to_string();
        let mut done: HashSet<i64> = self
            .0
            .prepare("SELECT task_id FROM completions WHERE date = ?1")
            .and_then(|mut st| {
                st.query_map(params![ds], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<_>>()
            })
            .unwrap_or_default();
        done.extend(self.once_done_ids());
        tasks.iter().any(|t| {
            if t.archived_at.is_some() || done.contains(&t.id) {
                return false;
            }
            if t.kind == TaskKind::Log || !occurs_today(t, d) && t.kind != TaskKind::Once {
                return false;
            }
            effective_due(t, d)
                .map(|due| due <= now + Duration::minutes(60))
                .unwrap_or(false)
        })
    }

    // ---------- 设置 ----------

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.0
            .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) {
        let _ = self.0.execute(
            "INSERT INTO settings (key, value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        );
    }

    /// 导出全部数据（备份用）
    pub fn export_json(&self) -> rusqlite::Result<String> {
        let tasks = self.all_tasks()?;
        let logs: Vec<(i64, String, Option<String>)> = {
            let mut st = self
                .0
                .prepare("SELECT task_id, created_at, note FROM log_entries ORDER BY id")?;
            let x = st
                .query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect::<rusqlite::Result<_>>()?;
            x
        };
        #[derive(serde::Serialize)]
        struct TaskOut {
            id: i64,
            kind: String,
            title: String,
            note: Option<String>,
            pinned: bool,
            deadline: Option<String>,
            weekly_days: Vec<u8>,
            interval_days: Option<i64>,
            daily_due_time: Option<String>,
            paused: bool,
            created_at: String,
            #[serde(rename = "type")]
            _t: &'static str,
        }
        let out: Vec<TaskOut> = tasks
            .iter()
            .map(|t| TaskOut {
                id: t.id,
                kind: kind_to_str(t.kind).into(),
                title: t.title.clone(),
                note: t.note.clone(),
                pinned: t.pinned,
                deadline: t.deadline.as_ref().map(fmt_datetime),
                weekly_days: t.weekly_days.clone(),
                interval_days: t.interval_days,
                daily_due_time: t.daily_due_time.map(|tm| tm.format("%H:%M").to_string()),
                paused: t.paused,
                created_at: fmt_datetime(&t.created_at),
                _t: "task",
            })
            .collect();
        #[derive(serde::Serialize)]
        struct Export {
            tasks: Vec<TaskOut>,
            log_entries: Vec<(i64, String, Option<String>)>,
        }
        serde_json::to_string_pretty(&Export {
            tasks: out,
            log_entries: logs,
        })
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Group, TaskKind};

    fn temp_db() -> Db {
        let dir = std::env::temp_dir().join(format!("edge-test-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        Db::open(&dir).unwrap()
    }

    #[test]
    fn once_task_stays_done_across_days() {
        let db = temp_db();
        let id = db
            .create_task(TaskKind::Once, "一次性", None, false, Some("2026-01-01"), &[], None, None)
            .unwrap();
        let d1 = chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let d2 = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();

        // 12-31 划掉
        db.toggle_complete(id, d1, true).unwrap();
        // 两天后的视图：必须仍显示已完成（Done 组），不得复活
        let items = db.view(d2).unwrap();
        let it = items.iter().find(|i| i.id == id).unwrap();
        assert!(it.completed && it.group == Group::Done);

        // 撤销划掉（任意一天）→ 恢复未完成
        db.toggle_complete(id, d2, false).unwrap();
        let items = db.view(d2).unwrap();
        let it = items.iter().find(|i| i.id == id).unwrap();
        assert!(!it.completed && it.group != Group::Done);
    }

    #[test]
    fn daily_task_resets_next_day() {
        let db = temp_db();
        let id = db.create_task(TaskKind::Daily, "每日", None, false, None, &[], None, None).unwrap();
        let d1 = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let d2 = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();

        db.toggle_complete(id, d1, true).unwrap();
        let items = db.view(d1).unwrap();
        assert!(items.iter().find(|i| i.id == id).unwrap().completed);
        // 次日自动刷新为未完成（重复任务的预期行为）
        let items = db.view(d2).unwrap();
        assert!(!items.iter().find(|i| i.id == id).unwrap().completed);
    }

    #[test]
    fn update_task_fields_persists_all() {
        let db = temp_db();
        let id = db
            .create_task(TaskKind::Once, "原标题", None, false, Some("2026-01-01"), &[], None, None)
            .unwrap();
        // 模拟编辑：改标题/类型/截止/置顶/周几/间隔/每日时刻
        db.update_task_fields(id, &[
            ("title".into(), "新标题".into()),
            ("kind".into(), "weekly".into()),
            ("deadline".into(), "2026-02-15T09:30".into()),
            ("pinned".into(), "1".into()),
            ("weekly_days".into(), "2,4".into()),
            ("interval_days".into(), "3".into()),
            ("daily_due_time".into(), "21:00".into()),
        ]).unwrap();

        let t = db.all_tasks().unwrap().into_iter().find(|t| t.id == id).unwrap();
        assert_eq!(t.title, "新标题");
        assert_eq!(t.kind, TaskKind::Weekly);
        assert!(t.pinned);
        assert_eq!(t.weekly_days, vec![2, 4]);
        assert_eq!(t.interval_days, Some(3));
        assert_eq!(
            t.daily_due_time.map(|tm| tm.format("%H:%M").to_string()),
            Some("21:00".into())
        );
        assert_eq!(
            t.deadline.map(|dt| dt.format("%Y-%m-%dT%H:%M").to_string()),
            Some("2026-02-15T09:30".into())
        );
    }
}
