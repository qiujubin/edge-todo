import { createMemo, createSignal, onCleanup, onMount, For, Show } from "solid-js";
import type { Group, TaskDraft, ViewItem } from "../types";
import { GROUP_LABEL, GROUP_ORDER } from "../types";
import { headerDate, isoDate, parseDue } from "../time";
import * as api from "../api";
import { TaskItem } from "./TaskItem";
import { LogItem } from "./LogItem";
import { EditDialog } from "./EditDialog";
import { IconPin, IconPlus, IconCollapse } from "./icons";

interface Props {
  items: ViewItem[];
  now: Date;
  locked: boolean;
  onToggleLock: () => void;
  onCollapse: () => void;
  onUndoableAction: (msg: string, undo: () => void) => void;
  /** 头部按下 → 系统级拖动（Rust 负责吸附） */
  onHeaderPress: (e: PointerEvent) => void;
}

/** 快捷语法："!" 置顶；"@明天/@周五/@12-01/@21:30" 截止（令牌可为中文） */
function parseQuick(raw: string): TaskDraft {
  let text = raw.replace(/\s+/g, " ").trim();
  let pinned = false;
  let date: string | null = null;
  let time: string | null = null;

  // "!" 作为独立词出现即置顶（开头/中置均可）
  if (/(^|\s)!(?=\s|$)/.test(text)) {
    pinned = true;
    text = text.replace(/(^|\s)!(?=\s|$)/g, "").trim();
  }

  // @令牌：@后接非空白字符（支持中文），尾部标点剔除
  const tokens = text.match(/@[^\s@，。,]+/g) ?? [];
  for (const tk of tokens) {
    const v = tk.slice(1).replace(/[.!?,，。;；]+$/, "");
    text = text.replace(tk, "").replace(/\s{2,}/g, " ").trim();
    if (/^\d{1,2}:\d{2}$/.test(v)) {
      time = v.padStart(5, "0");
    } else {
      date = resolveDateToken(v) ?? date;
    }
  }

  const deadline = date
    ? time
      ? `${date}T${time}`
      : date
    : time
      ? `${isoDate(new Date())}T${time}`
      : isoDate(new Date()); // 默认今天

  return { kind: "once", title: text, pinned, deadline };
}

function resolveDateToken(v: string): string | null {
  const now = new Date();
  const alias: Record<string, number> = {
    明天: 1,
    后天: 2,
    今天: 0,
  };
  if (v in alias) return isoDate(new Date(now.getTime() + alias[v] * 86400_000));
  const wd = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"].indexOf(v);
  if (wd >= 0) {
    const diff = (wd - now.getDay() + 7) % 7 || 7; // 下一个该日（今天则取下周）
    return isoDate(new Date(now.getTime() + diff * 86400_000));
  }
  const m = v.match(/^(\d{1,2})[-.．](\d{1,2})$/);
  if (m) {
    const month = parseInt(m[1], 10);
    const day = parseInt(m[2], 10);
    let year = now.getFullYear();
    const candidate = new Date(year, month - 1, day);
    if (candidate < now) year += 1; // 已过去则明年
    return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  }
  return null;
}

export function Panel(props: Props) {
  const [input, setInput] = createSignal("");
  const [editing, setEditing] = createSignal<ViewItem | null>(null);
  const [showDialog, setShowDialog] = createSignal(false);
  const [doneOpen, setDoneOpen] = createSignal(true);
  const [ctx, setCtx] = createSignal<{ x: number; y: number; item: ViewItem } | null>(null);

  const closeCtx = () => setCtx(null);
  /** 右键菜单：坐标钳制在窗口内 */
  const openCtx = (e: MouseEvent, item: ViewItem) => {
    e.preventDefault();
    e.stopPropagation();
    setCtx({
      x: Math.min(e.clientX, window.innerWidth - 150),
      y: Math.min(e.clientY, window.innerHeight - 210),
      item,
    });
  };

  onMount(() => {
    const onDocClick = () => closeCtx();
    const onDocCtx = (e: MouseEvent) => {
      if (!(e.target as HTMLElement).closest(".task")) closeCtx();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeCtx();
    };
    document.addEventListener("click", onDocClick);
    document.addEventListener("contextmenu", onDocCtx);
    window.addEventListener("keydown", onKey);
    onCleanup(() => {
      document.removeEventListener("click", onDocClick);
      document.removeEventListener("contextmenu", onDocCtx);
      window.removeEventListener("keydown", onKey);
    });
  });

  /** 右键菜单动作 */
  const ctxDelete = () => {
    const item = ctx()!.item;
    void api.deleteTask(item.id);
    props.onUndoableAction(`已删除「${item.title}」`, () =>
      api.createTask({
        kind: item.kind,
        title: item.title,
        note: item.note,
        pinned: item.pinned,
        deadline: item.dueAt,
      }),
    );
    closeCtx();
  };
  const ctxPin = () => {
    const item = ctx()!.item;
    void api.updateTask({ id: item.id, kind: item.kind, title: item.title, pinned: !item.pinned });
    closeCtx();
  };
  const ctxPostpone = () => {
    void api.postponeTask(ctx()!.item.id);
    closeCtx();
  };
  const ctxRecord = () => {
    void api.addLogEntry(ctx()!.item.id);
    closeCtx();
  };
  const ctxEdit = () => {
    setEditing(ctx()!.item);
    setShowDialog(true);
    closeCtx();
  };

  const date = createMemo(() => headerDate(props.now));
  const draft = createMemo(() => (input().trim() ? parseQuick(input().trim()) : null));

  const groups = createMemo(() => {
    const map = new Map<Group, ViewItem[]>();
    for (const it of props.items) {
      if (it.group === "done" && !doneOpen()) continue;
      const arr = map.get(it.group) ?? [];
      arr.push(it);
      map.set(it.group, arr);
    }
    return GROUP_ORDER.filter((g) => map.has(g) && map.get(g)!.length > 0).map((g) => ({
      group: g,
      items: map.get(g)!,
    }));
  });

  const stats = createMemo(() => {
    const dayItems = props.items.filter((i) => i.group !== "log" && i.group !== "later" && i.group !== "thisWeek" && i.group !== "tomorrow");
    const total = dayItems.length;
    const done = dayItems.filter((i) => i.completed).length;
    return { done, total };
  });

  const quickAdd = () => {
    const d = draft();
    if (!d || !d.title) return;
    setInput("");
    void api.createTask(d);
  };

  return (
    <div class="panel">
      <div class="panel-shell">
        <div class="panel-body">
          {/* 头部：日期 + 锁定/收起 */}
          <header class="panel-head" onPointerDown={props.onHeaderPress}>
            <div class="head-date">
              <span class="big tnum">{date().big}</span>
              <span class="sub">
                <span class="en">{date().en}</span>
                <span class="zh">{date().zh}</span>
              </span>
            </div>
            <div class="head-actions">
              <button
                class="icon-btn"
                classList={{ active: props.locked }}
                title={props.locked ? "取消常驻（将自动收起）" : "锁定常驻（不自动收起）"}
                onClick={props.onToggleLock}
              >
                <IconPin />
              </button>
              <button class="icon-btn" title="收起面板" onClick={props.onCollapse}>
                <IconCollapse />
              </button>
            </div>
          </header>

          {/* 快速添加 */}
          <div class="quick-add">
            <div class="quick-add-row">
              <button class="qa-submit" title="添加（回车）" aria-label="添加任务" onClick={quickAdd}>
                <IconPlus />
              </button>
              <input
                type="text"
                placeholder="添加任务…  !置顶  @明天  @周五  @21:30"
                value={input()}
                onInput={(e) => setInput(e.currentTarget.value)}
                onKeyDown={(e) => e.key === "Enter" && quickAdd()}
              />
              <button class="qa-detail" title="详细新建（选类型/重复规则）" aria-label="详细新建" onClick={() => { setEditing(null); setShowDialog(true); }}>
                <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 8 H13 M8 3 V13" opacity="0.55"/><path d="M2 2 L5 2 M2 2 L2 5 M14 14 L11 14 M14 14 L14 11" /></svg>
              </button>
            </div>
            <div class="parse-chips">
              <Show when={draft()}>
                <Show when={draft()!.pinned}>
                  <span class="chip">置顶</span>
                </Show>
                <Show when={draft()!.deadline}>
                  <span class="chip tnum">{dueChip(draft()!.deadline!, props.now)}</span>
                </Show>
              </Show>
            </div>
          </div>

          {/* 分组列表 */}
          <main class="task-scroll">
            <For each={groups()} fallback={<div class="empty-today"><span class="glyph">— EMPTY —</span>今天没有任务<br />输入框里写下第一件事</div>}>
              {(g) => (
                <section
                  class="group"
                  classList={{ "done-group": g.group === "done", collapsed: g.group === "done" && !doneOpen() }}
                >
                  <div
                    class="group-label"
                    onClick={() => g.group === "done" && setDoneOpen(!doneOpen())}
                  >
                    <Show when={g.group === "today" || g.group === "pinned"}>
                      <span class="tick" />
                    </Show>
                    <span class="en">{GROUP_LABEL[g.group].en}</span>
                    <span class="zh">{GROUP_LABEL[g.group].zh}</span>
                    <Show when={g.group === "done"}>
                      <span class="chev"><svg viewBox="0 0 16 16" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 6 L8 10 L12 6" /></svg></span>
                      <button
                        class="g-clear"
                        title="清除今日已完成（一次性归档）"
                        onClick={(e) => {
                          e.stopPropagation();
                          void api.clearCompletedToday();
                        }}
                      >
                        清除
                      </button>
                    </Show>
                    <span class="g-count tnum">{g.group === "done" ? `${g.items.length} 项` : `${g.items.length}`}</span>
                  </div>
                  <For each={g.items}>
                    {(item) => (
                      <Show when={item.kind !== "log"} fallback={
                        <LogItem item={item} onEdit={(i) => { setEditing(i); setShowDialog(true); }} onContextMenu={openCtx} />
                      }>
                        <TaskItem
                          item={item}
                          now={props.now}
                          onEdit={(i) => { setEditing(i); setShowDialog(true); }}
                          onUndoableAction={props.onUndoableAction}
                          onContextMenu={openCtx}
                        />
                      </Show>
                    )}
                  </For>
                </section>
              )}
            </For>
          </main>

          {/* 右键菜单 */}
          <Show when={ctx()}>
            <div class="ctx-menu" style={`left:${ctx()!.x}px;top:${ctx()!.y}px`}>
              <Show when={ctx()!.item.kind === "log"}>
                <button onClick={ctxRecord}>记一笔</button>
              </Show>
              <Show when={ctx()!.item.kind !== "log"}>
                <button onClick={ctxPostpone}>延到明天</button>
                <button onClick={ctxPin}>{ctx()!.item.pinned ? "取消置顶" : "置顶"}</button>
              </Show>
              <button onClick={ctxEdit}>编辑</button>
              <button class="danger" onClick={ctxDelete}>删除</button>
            </div>
          </Show>

          {/* 脚栏 */}
          <footer class="panel-foot">
            <span class="brand">EDGE<b>SPROUT</b></span>
            <span class="tnum">今日 {stats().done}/{stats().total}</span>
          </footer>
        </div>
      </div>

      <Show when={showDialog()}>
        <EditDialog
          editing={editing()}
          onClose={() => setShowDialog(false)}
        />
      </Show>
    </div>
  );
}

function dueChip(deadline: string, now: Date): string {
  const d = parseDue(deadline);
  if (!d) return deadline;
  const today = new Date(now);
  const diffDay = Math.round(
    (new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() -
      new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime()) /
      86400_000,
  );
  const hm = deadline.includes("T") ? ` ${deadline.slice(11, 16)}` : "";
  if (diffDay === 0) return `今天${hm}`;
  if (diffDay === 1) return `明天${hm}`;
  return `${d.getMonth() + 1}.${String(d.getDate()).padStart(2, "0")}${hm}`;
}
