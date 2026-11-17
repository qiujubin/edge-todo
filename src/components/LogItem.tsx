import { createSignal, For, Show } from "solid-js";
import type { LogEntry, ViewItem } from "../types";
import * as api from "../api";
import { IconChevron } from "./icons";

/** 流水记录型任务：常驻 + 「记一笔」+ 可展开历史时间线（占满整行） */
export function LogItem(props: {
  item: ViewItem;
  onEdit: (i: ViewItem) => void;
  onContextMenu: (e: MouseEvent, i: ViewItem) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [entries, setEntries] = createSignal<LogEntry[]>([]);
  const [flash, setFlash] = createSignal(false);

  const toggleHistory = async () => {
    const next = !open();
    setOpen(next);
    if (next) {
      setEntries(await api.getLogEntries(props.item.id));
    }
  };

  const record = async () => {
    setFlash(true);
    window.setTimeout(() => setFlash(false), 620);
    await api.addLogEntry(props.item.id);
    if (open()) {
      setEntries(await api.getLogEntries(props.item.id));
    }
  };

  return (
    <div
      class="task log-task"
      classList={{ flash: flash() }}
      onContextMenu={(e) => props.onContextMenu(e, props.item)}
    >
      <button class="log-plus" title="记一笔" onClick={() => void record()}>
        +
      </button>
      <div class="t-main">
        <div class="t-title-row">
          <span class="t-title">{props.item.title}</span>
        </div>
        <div class="t-badges">
          <span class="badge due tnum">今日 {props.item.logToday}</span>
          <span class="badge kind tnum">累计 {props.item.logTotal}</span>
        </div>
      </div>
      <div class="t-actions">
        <button
          class="icon-btn"
          title={open() ? "收起记录" : "展开记录"}
          onClick={() => void toggleHistory()}
        >
          <IconChevron />
        </button>
      </div>
      <Show when={open()}>
        <div class="log-history">
          <Show
            when={entries().length > 0}
            fallback={
              <div class="le" style="color: var(--color-text-faint)">
                还没有记录，点 + 记第一笔
              </div>
            }
          >
            <For each={entries().slice(0, 12)}>
              {(e) => (
                <div class="le">
                  <span class="at tnum">{e.at.slice(5, 16).replace("T", " ")}</span>
                  <span class="note">{e.note ?? ""}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  );
}
