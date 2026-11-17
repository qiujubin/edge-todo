# -*- coding: utf-8 -*-
"""给边芽 dev 库种一套覆盖全部任务类型的演示数据（幂等：先清空再插入）"""
import sqlite3
import os
import sys
from datetime import date, timedelta

db_path = os.path.join(os.environ["APPDATA"], "com.edgesprout.app", "edge.db")
conn = sqlite3.connect(db_path)
c = conn.cursor()

# 清空
for t in ["tasks", "completions", "postpones", "log_entries"]:
    c.execute(f"DELETE FROM {t}")
c.execute("DELETE FROM sqlite_sequence WHERE name IN ('tasks','log_entries')")

today = date.today()
tomorrow = today + timedelta(days=1)
overdue = today - timedelta(days=1)      # 已过期
friday = today + timedelta(days=(4 - today.weekday()) % 7 or 7)  # 下一个周五
far = today + timedelta(days=12)

def ins(kind, title, **kw):
    c.execute(
        "INSERT INTO tasks (kind,title,note,pinned,sort_order,deadline,weekly_days,interval_days,daily_due_time,paused,created_at)"
        " VALUES (?,?,?,?,?,?,?,?,?,?,?)",
        (
            kind, title, kw.get("note"), 1 if kw.get("pinned") else 0,
            kw.get("order", 0), kw.get("deadline"), kw.get("weekly_days"),
            kw.get("interval_days"), kw.get("daily_due_time"),
            1 if kw.get("paused") else 0,
            kw.get("created_at", f"{today - timedelta(days=20)}T09:00"),
        ),
    )
    return c.lastrowid

# 一次性：今日带时刻（临近 → warn/danger 徽章 + 呼吸灯）
ins("once", "买牛奶", deadline=f"{today}T21:30", order=10)
# 一次性：已过期 → 今天组 + 过期红徽章
ins("once", "给旧客户寄合同", deadline=f"{overdue}T17:00", order=11)
# 一次性：明天
ins("once", "给客户回电话", deadline=str(tomorrow), order=12)
# 一次性：本周五（置顶）
ins("once", "交周报", deadline=str(friday), pinned=True, order=13)
# 一次性：更远
ins("once", "续护照", deadline=str(far), order=14)

# 每日（带每日截止时刻 22:00）
daily_id = ins("daily", "背单词 30 个", daily_due_time="22:00", order=20)
# 每周：周三/周五
weekly_id = ins("weekly", "健身训练", weekly_days="3,5", order=21)
# 周期：每 3 天
interval_id = ins("interval", "浇花", interval_days=3, order=22)

# 流水记录型
log1 = ins("log", "喝水", note="每次一杯", order=30)
log2 = ins("log", "读书笔记", order=31)

# 完成状态：每日任务今日已完成 → DONE 组
c.execute("INSERT INTO completions (task_id,date,created_at) VALUES (?,?,?)",
          (daily_id, str(today), f"{today}T08:12"))

# 延期记录：一次性过期任务延过 3 次 → 拖延徽章
for _ in range(3):
    c.execute("INSERT INTO postpones (task_id,date,count) VALUES (?,?,1)"
              " ON CONFLICT(task_id,date) DO UPDATE SET count=count+1",
              (2, str(today)))

# 流水：喝水今天 3 笔、累计 9 笔；读书笔记 1 笔
marks = [(-3, "09:05"), (-2, "11:40"), (-2, "15:20"), (-5, "10:00"), (-4, "14:30"),
         (-1, "09:30"), (-1, "16:45"), (0, "08:50"), (0, "10:15"), (0, "14:05")]
for d, hm in marks:
    c.execute("INSERT INTO log_entries (task_id,created_at,note) VALUES (?,?,?)",
              (log1, f"{today + timedelta(days=d)}T{hm}", None))
c.execute("INSERT INTO log_entries (task_id,created_at,note) VALUES (?,?,?)",
          (log2, f"{today}T20:30", "第三章：索引优化"))

conn.commit()
n = c.execute("SELECT COUNT(*) FROM tasks").fetchone()[0]
print(f"seeded: {n} tasks, log_entries={c.execute('SELECT COUNT(*) FROM log_entries').fetchone()[0]}")
conn.close()
