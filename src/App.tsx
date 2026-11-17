import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import type { DockState, ViewItem } from "./types";
import * as api from "./api";
import { Panel } from "./components/Panel";

/** 自动收起节奏：离开面板 4s → 把手；把手再闲置 3s → 微光 */
const COLLAPSE_MS = 4000;
const HIDE_MS = 3000;

export function App() {
  const [dock, setDock] = createSignal<DockState>("hidden");
  const [items, setItems] = createSignal<ViewItem[]>([]);
  const [urgent, setUrgent] = createSignal(false);
  const [locked, setLocked] = createSignal(false);
  const [edge, setEdge] = createSignal<"left" | "right" | "top">("right");
  const [now, setNow] = createSignal(new Date());
  const [toast, setToast] = createSignal<{ msg: string; undo: (() => void) | null } | null>(null);

  let collapseTimer: number | undefined;
  let hideTimer: number | undefined;
  let toastTimer: number | undefined;

  const clearTimers = () => {
    window.clearTimeout(collapseTimer);
    window.clearTimeout(hideTimer);
  };

  /** 目标状态（含锁定判定）；展开面板时顺带刷新视图（跨零点/外部变更兜底） */
  const goTo = (s: DockState) => {
    clearTimers();
    void api.dockTo(s);
    setDock(s);
    if (s === "panel") {
      void api.getView().then((v) => {
        setItems(v.items);
        setUrgent(v.urgent);
      });
    }
  };

  /** 最近一次输入活动时间（打字保护：输入中不自动收起） */
  let lastInputAt = 0;
  const recentTyping = () =>
    Date.now() - lastInputAt < 10_000 ||
    !!document.querySelector(".dialog-scrim") ||
    !!document.querySelector(".ctx-menu");

  /** 离开窗口：延时收起（锁定/输入中不收） */
  const scheduleCollapse = () => {
    window.clearTimeout(collapseTimer);
    window.clearTimeout(hideTimer);
    if (locked() || dock() !== "panel") return;
    if (recentTyping()) return;
    collapseTimer = window.setTimeout(() => {
      if (locked() || recentTyping()) return;
      goTo("handle");
      hideTimer = window.setTimeout(() => {
        if (locked() || recentTyping()) return;
        goTo("hidden");
      }, HIDE_MS);
    }, COLLAPSE_MS);
  };

  const cancelCollapse = () => {
    window.clearTimeout(collapseTimer);
    window.clearTimeout(hideTimer);
  };

  /** 撤销型操作反馈 */
  const undoableAction = (msg: string, undo: () => void) => {
    setToast({ msg, undo });
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => setToast(null), 5000);
  };

  /** 头部按下 → 系统级拖动（Rust 侧负责吸附与持久化） */
  const onHeaderPress = (e: PointerEvent) => {
    if ((e.target as HTMLElement).closest("button, input")) return;
    cancelCollapse();
    void api.beginHeaderDrag();
  };

  onMount(async () => {
    // 初始数据
    const v = await api.getView();
    setItems(v.items);
    setUrgent(v.urgent);

    // Rust → 前端事件桥
    const un1 = await listen<DetailView>("view-updated", (e) => {
      setItems(e.payload.items);
      setUrgent(e.payload.urgent);
    });
    const un2 = await listen<string>("dock-changed", (e) => {
      setDock(e.payload as DockState);
    });
    const un3 = await listen<boolean>("urgent-changed", (e) => {
      setUrgent(e.payload);
    });
    const un4 = await listen<string>("dock-side-changed", (e) => {
      setEdge(e.payload as "left" | "right" | "top");
    });
    // Rust 动画开始即通知（托盘等非前端发起的收放也能同步 CSS 滑动）
    const un5 = await listen<string>("dock-anim", (e) => {
      setDock(e.payload as DockState);
    });

    // 停靠边初值
    setEdge(await api.getDockSide());

    // 输入活动追踪：输入框聚焦/按键时刷新时间戳
    const markInput = (e: Event) => {
      if ((e.target as HTMLElement).matches?.("input, textarea")) {
        lastInputAt = Date.now();
      }
    };
    window.addEventListener("keydown", markInput, true);
    window.addEventListener("focusin", markInput, true);

    // 窗口级鼠标事件（三态切换）
    document.addEventListener("mouseleave", () => scheduleCollapse());
    document.addEventListener("mouseenter", () => cancelCollapse());
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && dock() === "panel") goTo("handle");
    });

    // 分钟级时钟（倒计时徽章）
    const clock = window.setInterval(() => setNow(new Date()), 30_000);

    // 锁定状态持久化恢复
    const saved = await api.getSetting("locked");
    if (saved === "1") setLocked(true);

    onCleanup(() => {
      un1();
      un2();
      un3();
      un4();
      un5();
      window.clearInterval(clock);
      window.removeEventListener("keydown", markInput, true);
      window.removeEventListener("focusin", markInput, true);
    });
  });

  const toggleLock = async () => {
    const next = !locked();
    setLocked(next);
    await api.setSetting("locked", next ? "1" : "0");
    if (!next && dock() === "panel") scheduleCollapse();
  };

  const todayCount = () => {
    const undone = items().filter(
      (i) => !i.completed && i.group !== "log" && i.group !== "done",
    ).length;
    return undone;
  };

  return (
    <div
      class="app-root"
      classList={{
        "dock-left": edge() === "left",
        "dock-top": edge() === "top",
        "st-hidden": dock() === "hidden",
        "st-handle": dock() === "handle",
        "st-panel": dock() === "panel",
      }}
      style="position: relative; width: 100%; height: 100%;"
      onMouseEnter={() => {
        // 进入窗口可见区即展开（Hidden/Handle → Panel）
        if (dock() !== "panel") goTo("panel");
      }}
    >
      {/* 微光线（Hidden 态的脸面） */}
      <div
        class="glow-strip"
        classList={{ urgent: urgent() && dock() === "hidden" }}
        onClick={() => goTo("panel")}
      />

      {/* 把手（Handle 态的脸面） */}
      <div
        class="handle"
        classList={{ urgent: urgent() && dock() !== "panel" }}
        onClick={() => goTo("panel")}
      >
        <span class="h-date tnum">{handleDate()}</span>
        <span class="h-mark" />
        <span class="h-count tnum" classList={{ zero: todayCount() === 0 }}>
          {todayCount()}
        </span>
      </div>

      {/* 面板 */}
      <Show when={dock() === "panel" || dock() === "handle"}>
        <Panel
          items={items()}
          now={now()}
          locked={locked()}
          onToggleLock={() => void toggleLock()}
          onCollapse={() => goTo("handle")}
          onUndoableAction={undoableAction}
          onHeaderPress={onHeaderPress}
        />
      </Show>

      {/* 撤销 Toast */}
      <Show when={toast()}>
        <div class="toast">
          <span>{toast()!.msg}</span>
          <Show when={toast()!.undo}>
            <button
              onClick={() => {
                toast()!.undo?.();
                setToast(null);
              }}
            >
              撤销
            </button>
          </Show>
        </div>
      </Show>
    </div>
  );
}

interface DetailView {
  items: ViewItem[];
  urgent: boolean;
}

function handleDate(): string {
  const d = new Date();
  return `${String(d.getMonth() + 1).padStart(2, "0")}.${String(d.getDate()).padStart(2, "0")}`;
}
