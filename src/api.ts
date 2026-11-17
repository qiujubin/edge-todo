import { invoke } from "@tauri-apps/api/core";
import type { LogEntry, TaskDraft, ViewData } from "./types";

export async function getView(): Promise<ViewData> {
  return invoke<ViewData>("get_view");
}

export async function createTask(draft: TaskDraft): Promise<number> {
  return invoke<number>("create_task", {
    kind: draft.kind,
    title: draft.title,
    note: draft.note ?? null,
    pinned: draft.pinned,
    deadline: draft.deadline ?? null,
    weeklyDays: draft.weeklyDays ?? [],
    intervalDays: draft.intervalDays ?? null,
    dailyDueTime: draft.dailyDueTime ?? null,
  });
}

export async function updateTask(draft: TaskDraft): Promise<void> {
  await invoke("update_task", {
    id: draft.id,
    title: draft.title,
    note: draft.note ?? null,
    pinned: draft.pinned,
    deadline: draft.deadline ?? null,
    weeklyDays: draft.weeklyDays ?? null,
    intervalDays: draft.intervalDays ?? null,
    dailyDueTime: draft.dailyDueTime ?? null,
    paused: null,
  });
}

export async function deleteTask(id: number): Promise<void> {
  await invoke("delete_task", { id });
}

export async function toggleComplete(id: number, done: boolean): Promise<void> {
  await invoke("toggle_complete", { id, done });
}

export async function postponeTask(id: number): Promise<void> {
  await invoke("postpone_task", { id });
}

export async function addLogEntry(id: number, note?: string): Promise<void> {
  await invoke("add_log_entry", { id, note: note ?? null });
}

export async function getLogEntries(id: number): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_log_entries", { id });
}

export async function clearCompletedToday(): Promise<void> {
  await invoke("clear_completed_today");
}

export async function dockTo(target: "hidden" | "handle" | "panel"): Promise<void> {
  await invoke("dock_to", { target });
}

/** 头部拖动：系统级拖动，松手后 Rust 侧吸附最近的屏幕左右缘 */
export async function beginHeaderDrag(): Promise<void> {
  await invoke("begin_header_drag");
}

export async function setDockSide(side: "left" | "right" | "top"): Promise<void> {
  await invoke("set_dock_side", { side });
}

export async function getDockSide(): Promise<"left" | "right" | "top"> {
  return invoke<"left" | "right" | "top">("get_dock_side");
}

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  await invoke("set_setting", { key, value });
}
