import { createSignal, Show } from "solid-js";
import { dueLabel } from "../time";
import type { ViewItem } from "../types";
import * as api from "../api";
import { IconPin, IconPostpone, IconEdit, IconTrash } from "./icons";

interface Props {
  item: ViewItem;
  now: Date;
  onEdit: (item: ViewItem) => void;
  onUndoableAction: (msg: string, undo: () => void) => void;
  onContextMenu: (e: MouseEvent, item: ViewItem) => void;
}

export function TaskItem(props: Props) {
  const [striking, setStriking] = createSignal(false);
  const due = () => (props.item.dueAt ? dueLabel(props.item.dueAt, props.now) : null);

  /** 划掉：笔划动画 → 落库 → 可撤销 */
  const strike = () => {
    if (props.item.completed) {
      // 已完成 → 撤销划掉
      void api.toggleComplete(props.item.id, false);
      return;
    }
    if (striking()) return;
    setStriking(true);
    window.setTimeout(() => {
      void api.toggleComplete(props.item.id, true);
      props.onUndoableAction(`已完成「${props.item.title}」`, () =>
        api.toggleComplete(props.item.id, false),
      );
    }, 380);
  };

  return (
    <div
      class="task"
      classList={{ completed: props.item.completed, striking: striking() }}
      data-id={props.item.id}
      onContextMenu={(e) => props.onContextMenu(e, props.item)}
    >
      <button class="check" aria-label="完成" onClick={strike} />

      <div class="t-main">
        <div class="t-title-row">
          <span class="t-title">
            {props.item.title}
            <svg class="strike-svg" viewBox="0 0 300 24" preserveAspectRatio="none">
              <path d="M2 14 C60 10, 120 16, 180 11 S 280 13, 298 10" />
            </svg>
          </span>
        </div>
        <Show when={props.item.kind !== "once" || props.item.dueAt || props.item.postponeCount >= 3}>
          <div class="t-badges">
            <Show when={props.item.kind !== "once"}>
              <span class="badge kind">{kindTag(props.item)}</span>
            </Show>
            <Show when={due()}>
              <span class="badge due" classList={{ warn: due()!.tone === "warn", danger: due()!.tone === "danger" }}>
                {due()!.text}
              </span>
            </Show>
            <Show when={props.item.overdue && !due()?.text.startsWith("过")}>
              <span class="badge due danger">过期</span>
            </Show>
            <Show when={props.item.postponeCount >= 3}>
              <span class="badge procrast" title={`已延期 ${props.item.postponeCount} 次`}>
                拖延×{props.item.postponeCount}
              </span>
            </Show>
          </div>
        </Show>
      </div>

      <div class="t-actions">
        <button class="icon-btn" title="置顶" onClick={() => void api.updateTask({ id: props.item.id, kind: props.item.kind, title: props.item.title, pinned: !props.item.pinned })}>
          <IconPin />
        </button>
        <Show when={props.item.kind === "once" || occursRecurring(props.item)}>
          <button
            class="icon-btn"
            title="延到明天"
            onClick={() => {
              void api.postponeTask(props.item.id);
              props.onUndoableAction(`已延期「${props.item.title}」`, () => {
                // 延期不支持一键撤销（语义复杂），提示里说明
              });
            }}
          >
            <IconPostpone />
          </button>
        </Show>
        <button class="icon-btn" title="编辑" onClick={() => props.onEdit(props.item)}>
          <IconEdit />
        </button>
        <button
          class="icon-btn danger"
          title="删除"
          onClick={() => {
            const snapshot = props.item;
            void api.deleteTask(props.item.id);
            // 撤销重建需携带完整周期配置（weekly/interval）
            props.onUndoableAction(`已删除「${snapshot.title}」`, () =>
              api.createTask({
                kind: snapshot.kind,
                title: snapshot.title,
                note: snapshot.note,
                pinned: snapshot.pinned,
                deadline: snapshot.dueAt,
                weeklyDays: snapshot.weeklyDays,
                intervalDays: snapshot.intervalDays,
              }),
            );
          }}
        >
          <IconTrash />
        </button>
      </div>
    </div>
  );
}

function kindTag(item: ViewItem): string {
  switch (item.kind) {
    case "daily":
      return "每日";
    case "weekly":
      return "每周";
    case "interval":
      return "周期";
    case "log":
      return "流水";
    default:
      return "";
  }
}

/** 重复任务也允许"今日延期" */
function occursRecurring(_item: ViewItem): boolean {
  return _item.kind === "daily" || _item.kind === "weekly" || _item.kind === "interval";
}
