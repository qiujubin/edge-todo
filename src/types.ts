export type TaskKind = "once" | "daily" | "weekly" | "interval" | "log";

export type Group =
  | "pinned"
  | "today"
  | "tomorrow"
  | "thisWeek"
  | "later"
  | "log"
  | "done";

export interface ViewItem {
  id: number;
  kind: TaskKind;
  title: string;
  note: string | null;
  pinned: boolean;
  sortOrder: number;
  group: Group;
  completed: boolean;
  overdue: boolean;
  /** ISO "2026-11-20T18:00" 或 null */
  dueAt: string | null;
  postponeCount: number;
  logToday: number;
  logTotal: number;
  /** weekly：周几列表（编辑回显 / 删除撤销重建用） */
  weeklyDays: number[];
  /** interval：间隔天数 */
  intervalDays: number | null;
}

export interface ViewData {
  items: ViewItem[];
  urgent: boolean;
  date: string;
}

export interface LogEntry {
  at: string;
  note: string | null;
}

export type DockState = "hidden" | "handle" | "panel";

/** 编辑器负载（新建/编辑共用） */
export interface TaskDraft {
  id?: number;
  kind: TaskKind;
  title: string;
  note?: string | null;
  pinned: boolean;
  /** once 截止 "2026-11-20" 或 "2026-11-20T18:00" */
  deadline?: string | null;
  weeklyDays?: number[];
  intervalDays?: number | null;
  dailyDueTime?: string | null;
}

export const KIND_LABEL: Record<TaskKind, string> = {
  once: "一次性",
  daily: "每日",
  weekly: "每周",
  interval: "周期",
  log: "流水",
};

export const GROUP_LABEL: Record<Group, { en: string; zh: string }> = {
  pinned: { en: "PINNED", zh: "置顶" },
  today: { en: "TODAY", zh: "今天" },
  tomorrow: { en: "TOMORROW", zh: "明天" },
  thisWeek: { en: "THIS WEEK", zh: "本周" },
  later: { en: "LATER", zh: "稍后" },
  log: { en: "LOG", zh: "流水记录" },
  done: { en: "DONE", zh: "已完成" },
};

export const GROUP_ORDER: Group[] = [
  "pinned",
  "today",
  "tomorrow",
  "thisWeek",
  "later",
  "log",
  "done",
];
