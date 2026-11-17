import { createSignal, For, Show } from "solid-js";
import type { TaskDraft, TaskKind, ViewItem } from "../types";
import { KIND_LABEL } from "../types";
import { isoDate } from "../time";
import * as api from "../api";

const KINDS: TaskKind[] = ["once", "daily", "weekly", "interval", "log"];
const WEEK = ["一", "二", "三", "四", "五", "六", "日"];

interface Props {
  /** 编辑已有任务时传入 */
  editing: ViewItem | null;
  onClose: () => void;
}

export function EditDialog(props: Props) {
  const src = props.editing;
  const [kind, setKind] = createSignal<TaskKind>(src?.kind ?? "once");
  const [title, setTitle] = createSignal(src?.title ?? "");
  const [note, setNote] = createSignal(src?.note ?? "");
  const [pinned, setPinned] = createSignal(src?.pinned ?? false);

  // once 截止
  const [dueDate, setDueDate] = createSignal(src?.dueAt?.slice(0, 10) ?? isoDate(new Date()));
  const [dueTime, setDueTime] = createSignal(src?.dueAt?.slice(11, 16) ?? "");
  // weekly
  const [days, setDays] = createSignal<number[]>(parseWeekly(src));
  // interval
  const [every, setEvery] = createSignal<number>(2);
  // 重复任务每日截止时刻
  const [dailyTime, setDailyTime] = createSignal("");

  const toggleDay = (d: number) => {
    setDays((prev) => (prev.includes(d) ? prev.filter((x) => x !== d) : [...prev, d].sort()));
  };

  const save = async () => {
    const t = title().trim();
    if (!t) return;
    const deadline =
      kind() === "once" ? buildDeadline(dueDate(), dueTime()) : null;
    const draft: TaskDraft = {
      id: src?.id,
      kind: kind(),
      title: t,
      note: note().trim() || null,
      pinned: pinned(),
      deadline,
      weeklyDays:
        kind() === "weekly"
          ? days().length
            ? days()
            : src?.kind === "weekly"
              ? undefined // 编辑时未改动 → 不覆盖原值
              : [new Date().getDay() === 0 ? 7 : new Date().getDay()] // 新建默认今天
          : undefined,
      intervalDays: kind() === "interval" ? every() : null,
      dailyDueTime:
        kind() !== "once" && kind() !== "log" && dailyTime() ? dailyTime() : null,
    };
    if (src) {
      await api.updateTask(draft);
    } else {
      await api.createTask(draft);
    }
    props.onClose();
  };

  return (
    <div class="dialog-scrim" onClick={(e) => e.target === e.currentTarget && props.onClose()}>
      <div class="dialog" role="dialog" aria-label={src ? "编辑任务" : "新建任务"}>
        <div class="dialog-inner">
          <h3>{src ? "EDIT · 变更任务" : "NEW · 新任务"}</h3>

          <div class="field">
            <label>TITLE · 标题</label>
            <input
              type="text"
              value={title()}
              placeholder="要做什么？"
              onInput={(e) => setTitle(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && void save()}
              autofocus
            />
          </div>

          <div class="field">
            <label>KIND · 类型</label>
            <div class="seg">
              <For each={KINDS}>
                {(k) => (
                  <button classList={{ on: kind() === k }} onClick={() => setKind(k)}>
                    {KIND_LABEL[k]}
                  </button>
                )}
              </For>
            </div>
          </div>

          <Show when={kind() === "once"}>
            <div class="row2">
              <div class="field">
                <label>DEADLINE · 截止日期</label>
                <input type="date" value={dueDate()} onInput={(e) => setDueDate(e.currentTarget.value)} />
              </div>
              <div class="field">
                <label>TIME · 时刻（可选）</label>
                <input type="time" value={dueTime()} onInput={(e) => setDueTime(e.currentTarget.value)} />
              </div>
            </div>
          </Show>

          <Show when={kind() === "weekly"}>
            <div class="field">
              <label>REPEAT · 重复于</label>
              <div class="week-days">
                <For each={WEEK}>
                  {(w, i) => (
                    <button classList={{ on: days().includes(i() + 1) }} onClick={() => toggleDay(i() + 1)}>
                      {w}
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>

          <Show when={kind() === "interval"}>
            <div class="field">
              <label>EVERY · 每隔几天</label>
              <input
                type="number"
                min="1"
                max="365"
                value={every()}
                onInput={(e) => setEvery(Math.max(1, parseInt(e.currentTarget.value || "1", 10)))}
              />
            </div>
          </Show>

          <Show when={kind() === "daily" || kind() === "weekly" || kind() === "interval"}>
            <div class="field">
              <label>DAILY DUE · 每日截止时刻（可选）</label>
              <input type="time" value={dailyTime()} onInput={(e) => setDailyTime(e.currentTarget.value)} />
            </div>
          </Show>

          <Show when={kind() === "log"}>
            <div class="field">
              <label>FLOW · 流水说明</label>
              <input
                type="text"
                value={note()}
                placeholder="不定期多次发生的事，如：喝水 / 读书记录 / 灵感"
                onInput={(e) => setNote(e.currentTarget.value)}
              />
            </div>
          </Show>

          <Show when={kind() !== "log"}>
            <div class="field">
              <label>NOTE · 备注（可选）</label>
              <textarea value={note()} onInput={(e) => setNote(e.currentTarget.value)} />
            </div>
          </Show>

          <div class="toggle-line">
            <span>置顶显示</span>
            <button class="switch" classList={{ on: pinned() }} role="switch" aria-checked={pinned()} onClick={() => setPinned(!pinned())} />
          </div>

          <div class="dialog-actions">
            <Show when={src}>
              <button
                class="btn danger"
                onClick={() => {
                  if (src) void api.deleteTask(src.id);
                  props.onClose();
                }}
              >
                删除
              </button>
            </Show>
            <button class="btn ghost" onClick={props.onClose}>
              取消
            </button>
            <button class="btn primary" onClick={() => void save()}>
              {src ? "保存变更" : "创建"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function buildDeadline(date: string, time: string): string {
  if (!date) return isoDate(new Date());
  return time ? `${date}T${time}` : date;
}

function parseWeekly(_src: ViewItem | null): number[] {
  // ViewItem 不含 weekly_days 明细，编辑时先显示空选，保存时空值不覆盖
  return [];
}
