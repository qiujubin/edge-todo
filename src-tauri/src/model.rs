use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

/// 任务类型（五种）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    /// 一次性：有截止
    Once,
    /// 每日
    Daily,
    /// 每周：绑定周几
    Weekly,
    /// 周期：每隔 N 天
    Interval,
    /// 流水记录：不定期多次记一笔
    Log,
}

/// 数据库中的任务行
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub kind: TaskKind,
    pub title: String,
    pub note: Option<String>,
    pub pinned: bool,
    pub sort_order: f64,
    /// once: 截止日期（可含时间），ISO: "2026-11-20" 或 "2026-11-20T18:00"
    pub deadline: Option<NaiveDateTime>,
    /// weekly: 周几列表（1=周一..7=周日）
    pub weekly_days: Vec<u8>,
    /// interval: 间隔天数
    pub interval_days: Option<i64>,
    /// interval: 锚点日期（改间隔时重置；缺省回退创建日）
    pub interval_anchor: Option<NaiveDate>,
    /// 重复任务可选的每日截止时刻 "22:00"
    pub daily_due_time: Option<NaiveTime>,
    /// log 型暂停
    pub paused: bool,
    pub created_at: NaiveDateTime,
    pub archived_at: Option<NaiveDateTime>,
}

/// 前端展示分组
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Pinned,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
    Log,
    Done,
}

/// 视图条目：任务 + 在指定日期的呈现状态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewItem {
    pub id: i64,
    pub kind: TaskKind,
    pub title: String,
    pub note: Option<String>,
    pub pinned: bool,
    pub sort_order: f64,
    pub group: Group,
    /// 今日是否已完成（划掉）
    pub completed: bool,
    /// 已过期（针对带截止的）
    pub overdue: bool,
    /// 有效到期时刻（本地 ISO："2026-11-20T18:00" 或仅日期），前端算倒计时
    pub due_at: Option<String>,
    pub postpone_count: i64,
    /// log 型：今日已记笔数
    pub log_today: i64,
    /// log 型：累计笔数
    pub log_total: i64,
    /// weekly：周几列表（编辑回显 / 删除撤销重建用）
    pub weekly_days: Vec<u8>,
    /// interval：间隔天数
    pub interval_days: Option<i64>,
}

// ---------- 日期规则推导 ----------

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// 重复任务今天是否出现
pub fn occurs_today(t: &TaskRow, d: NaiveDate) -> bool {
    match t.kind {
        TaskKind::Daily => true,
        TaskKind::Weekly => d
            .weekday()
            .num_days_from_monday()
            .checked_add(1)
            .map(|n| t.weekly_days.contains(&(n as u8)))
            .unwrap_or(false),
        TaskKind::Interval => {
            let n = t.interval_days.unwrap_or(1).max(1) as i64;
            let anchor = t.interval_anchor.unwrap_or_else(|| t.created_at.date());
            let diff = (d - anchor).num_days();
            diff >= 0 && diff % n == 0
        }
        _ => false,
    }
}

/// 周一为首日，今天所在周的最后一堂（周日）
fn end_of_week(d: NaiveDate) -> NaiveDate {
    d + Duration::days((6 - d.weekday().num_days_from_monday()) as i64)
}

/// 推导任务在日期 d 应属的分组（未考虑完成/置顶覆盖）
fn natural_group(t: &TaskRow, d: NaiveDate) -> Option<Group> {
    match t.kind {
        TaskKind::Once => {
            let dl = t.deadline?.date();
            Some(if dl < d {
                Group::Today // 过期的一次性任务压到今天
            } else if dl == d {
                Group::Today
            } else if dl == d + Duration::days(1) {
                Group::Tomorrow
            } else if dl <= end_of_week(d) {
                Group::ThisWeek
            } else {
                Group::Later
            })
        }
        TaskKind::Daily | TaskKind::Weekly | TaskKind::Interval => {
            if occurs_today(t, d) {
                Some(Group::Today)
            } else {
                None // 今天不出现的重复任务不展示
            }
        }
        TaskKind::Log => Some(Group::Log),
    }
}

/// 今日有效到期时刻（重复任务用 daily_due_time 拼今天日期）
pub fn effective_due(t: &TaskRow, d: NaiveDate) -> Option<NaiveDateTime> {
    match t.kind {
        TaskKind::Once => t.deadline,
        TaskKind::Daily | TaskKind::Weekly | TaskKind::Interval => {
            if !occurs_today(t, d) {
                return None;
            }
            t.daily_due_time.map(|tm| NaiveDateTime::new(d, tm))
        }
        TaskKind::Log => None,
    }
}

/// 构建某日视图。completed/postponed/log_today 为当日记录集合；
/// done_once 为「已有任意完成记录的一次性任务」——一次性任务是二态，跨天不复活。
pub fn build_view(
    tasks: &[TaskRow],
    d: NaiveDate,
    completed: &std::collections::HashSet<i64>,
    done_once: &std::collections::HashSet<i64>,
    postponed: &std::collections::HashSet<i64>,
    log_today: &std::collections::HashMap<i64, i64>,
    log_total: &std::collections::HashMap<i64, i64>,
    postpone_counts: &std::collections::HashMap<i64, i64>,
) -> Vec<ViewItem> {
    let now = Local::now().naive_local();
    let mut out = Vec::new();
    for t in tasks {
        if t.archived_at.is_some() {
            continue;
        }
        let is_done =
            completed.contains(&t.id) || (t.kind == TaskKind::Once && done_once.contains(&t.id));
        let group = if t.kind == TaskKind::Log {
            if t.paused {
                continue;
            }
            Some(Group::Log)
        } else {
            natural_group(t, d)
        };
        let Some(mut group) = group else { continue };

        // 重复任务今日被延期 → 今天不显示
        if t.kind != TaskKind::Once
            && t.kind != TaskKind::Log
            && postponed.contains(&t.id)
            && !is_done
        {
            continue;
        }

        if is_done {
            group = Group::Done;
        } else if t.pinned && group != Group::Done {
            group = Group::Pinned;
        }

        let due = effective_due(t, d);
        let overdue = !is_done
            && due.map(|dt| dt < now).unwrap_or(false)
            && t.kind == TaskKind::Once;

        out.push(ViewItem {
            id: t.id,
            kind: t.kind,
            title: t.title.clone(),
            note: t.note.clone(),
            pinned: t.pinned,
            sort_order: t.sort_order,
            group,
            completed: is_done,
            overdue,
            due_at: due.map(|dt| dt.format("%Y-%m-%dT%H:%M").to_string()),
            postpone_count: postpone_counts.get(&t.id).copied().unwrap_or(0),
            log_today: log_today.get(&t.id).copied().unwrap_or(0),
            log_total: log_total.get(&t.id).copied().unwrap_or(0),
            weekly_days: t.weekly_days.clone(),
            interval_days: t.interval_days,
        });
    }
    out.sort_by(|a, b| a.sort_order.partial_cmp(&b.sort_order).unwrap_or(std::cmp::Ordering::Equal));
    out
}

// ---------- 序列化辅助 ----------

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Some(dt);
    }
    parse_date(s).map(|d| NaiveDateTime::new(d, NaiveTime::from_hms_opt(23, 59, 0).unwrap()))
}

pub fn fmt_datetime(dt: &NaiveDateTime) -> String {
    if dt.time() == NaiveTime::from_hms_opt(23, 59, 0).unwrap() {
        dt.format("%Y-%m-%d").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M").to_string()
    }
}

pub fn weekday_cn(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "一",
        Weekday::Tue => "二",
        Weekday::Wed => "三",
        Weekday::Thu => "四",
        Weekday::Fri => "五",
        Weekday::Sat => "六",
        Weekday::Sun => "日",
    }
}
