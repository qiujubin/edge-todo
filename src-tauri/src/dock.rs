//! 贴边停靠 v4：窗口矩形永远完整落在停靠屏工作区内，可见性由 Win32 窗口区域
//! （SetWindowRgn）裁剪控制。支持任意显示器的 左缘/右缘/上缘 停靠：
//! 拖动面板头松手吸附最近的边；显示器/边/位置持久化，重启恢复。
//! 所有状态切换前都重申贴边几何，保证收起时精确贴住所在边缘。

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use crate::db::Db;

/// 各状态露出的窄条宽度（逻辑 px；面板态 = 全部可见）
const STRIP_HIDDEN: f64 = 8.0;
const STRIP_HANDLE: f64 = 44.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockState {
    Hidden,
    Handle,
    Panel,
}

impl DockState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockState::Hidden => "hidden",
            DockState::Handle => "handle",
            DockState::Panel => "panel",
        }
    }
    fn from_u8(v: u8) -> Self {
        match v {
            1 => DockState::Handle,
            2 => DockState::Panel,
            _ => DockState::Hidden,
        }
    }
    fn strip_px(&self, scale: f64, edge: Edge, full_w: i32, full_h: i32) -> i32 {
        match self {
            DockState::Hidden => (STRIP_HIDDEN * scale).round() as i32,
            DockState::Handle => (STRIP_HANDLE * scale).round() as i32,
            // 面板态"全显"：左右缘取全宽，上缘取全高
            DockState::Panel => match edge {
                Edge::Top => full_h,
                _ => full_w,
            },
        }
    }
}

/// 停靠边（底部不做：任务栏常在底部）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
}

impl Edge {
    pub fn as_str(&self) -> &'static str {
        match self {
            Edge::Left => "left",
            Edge::Right => "right",
            Edge::Top => "top",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "left" => Edge::Left,
            "top" => Edge::Top,
            _ => Edge::Right,
        }
    }
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Edge::Left,
            2 => Edge::Top,
            _ => Edge::Right,
        }
    }
}

struct DockInner {
    gen: AtomicU64,
    animating: AtomicBool,
    state: AtomicU8,
    edge: AtomicU8,
    cur_strip: AtomicI32,
    cur_x: AtomicI32,
    cur_y: AtomicI32,
    wa_x: AtomicI32,
    wa_y: AtomicI32,
    wa_w: AtomicI32,
    wa_h: AtomicI32,
    scale_milli: AtomicU32,
    ready: AtomicBool,
}

#[derive(Clone)]
pub struct Dock {
    inner: Arc<DockInner>,
    db: Arc<Mutex<Db>>,
}

#[cfg(windows)]
mod win32 {
    #[link(name = "gdi32")]
    extern "system" {
        pub fn CreateRectRgn(x1: i32, y1: i32, x2: i32, y2: i32) -> isize;
    }
    #[link(name = "user32")]
    extern "system" {
        pub fn SetWindowRgn(hwnd: isize, rgn: isize, redraw: i32) -> i32;
    }
}

/// 窄条矩形（窗口本地物理坐标）
fn region_rect(edge: Edge, w: i32, h: i32, strip: i32) -> (i32, i32, i32, i32) {
    match edge {
        Edge::Left => (0, 0, strip.clamp(0, w), h),
        Edge::Right => ((w - strip).max(0), 0, w, h),
        Edge::Top => (0, 0, w, strip.clamp(0, h)),
    }
}

fn apply_region_of(inner: &DockInner, win: &WebviewWindow, strip: i32) {
    #[cfg(windows)]
    {
        let Ok(hh) = win.hwnd() else { return };
        let hwnd = hh.0 as isize;
        let Ok(size) = win.inner_size() else { return };
        let (w, ht) = (size.width as i32, size.height as i32);
        if w <= 0 || ht <= 0 {
            return;
        }
        let edge = Edge::from_u8(inner.edge.load(Ordering::SeqCst));
        let (x1, y1, x2, y2) = region_rect(edge, w, ht, strip);
        unsafe {
            let rgn = win32::CreateRectRgn(x1, y1, x2, y2);
            win32::SetWindowRgn(hwnd, rgn, 1);
        }
    }
}

impl Dock {
    pub fn new(db: Arc<Mutex<Db>>) -> Self {
        Self {
            inner: Arc::new(DockInner {
                gen: AtomicU64::new(0),
                animating: AtomicBool::new(false),
                state: AtomicU8::new(0),
                edge: AtomicU8::new(1),
                cur_strip: AtomicI32::new(0),
                cur_x: AtomicI32::new(i32::MIN),
                cur_y: AtomicI32::new(i32::MIN),
                wa_x: AtomicI32::new(0),
                wa_y: AtomicI32::new(0),
                wa_w: AtomicI32::new(0),
                wa_h: AtomicI32::new(0),
                scale_milli: AtomicU32::new(1000),
                ready: AtomicBool::new(false),
            }),
            db,
        }
    }

    fn persist(&self, name: &str, edge: Edge, x: i32, y: i32) {
        if let Ok(db) = self.db.lock() {
            db.set_setting("dock_monitor", name);
            db.set_setting("dock_side", edge.as_str());
            db.set_setting("dock_x", &x.to_string());
            db.set_setting("dock_y", &y.to_string());
        }
    }

    fn work_area(&self) -> (i32, i32, i32, i32, f64) {
        let scale = self.inner.scale_milli.load(Ordering::SeqCst) as f64 / 1000.0;
        (
            self.inner.wa_x.load(Ordering::SeqCst),
            self.inner.wa_y.load(Ordering::SeqCst),
            self.inner.wa_w.load(Ordering::SeqCst),
            self.inner.wa_h.load(Ordering::SeqCst),
            scale,
        )
    }

    pub fn edge(&self) -> Edge {
        Edge::from_u8(self.inner.edge.load(Ordering::SeqCst))
    }

    fn cur_state(&self) -> DockState {
        DockState::from_u8(self.inner.state.load(Ordering::SeqCst))
    }

    fn apply_region(&self, win: &WebviewWindow, strip: i32) {
        apply_region_of(&self.inner, win, strip);
    }

    /// 当前边"全显"对应的尺寸（左右缘=全宽，上缘=全高）
    fn full_strip(&self, win: &WebviewWindow) -> i32 {
        let Ok(size) = win.inner_size() else { return 0 };
        match self.edge() {
            Edge::Top => size.height as i32,
            _ => size.width as i32,
        }
    }

    /// 防御：把窗口重申到缓存工作区的精确贴边位置（x/y 按边语义钳制）
    fn assert_flush(&self, win: &WebviewWindow) {
        let (wx, wy, ww, wh, _) = self.work_area();
        let Ok(size) = win.inner_size() else { return };
        let (w, h) = (size.width as i32, size.height as i32);
        let edge = self.edge();
        let y_clamp = |v: i32| v.clamp(wy, (wy + wh - h).max(wy));
        let x_clamp = |v: i32| v.clamp(wx, (wx + ww - w).max(wx));
        let (x, y) = match edge {
            Edge::Left => (wx, y_clamp(self.inner.cur_y.load(Ordering::SeqCst))),
            Edge::Right => (wx + ww - w, y_clamp(self.inner.cur_y.load(Ordering::SeqCst))),
            Edge::Top => (x_clamp(self.inner.cur_x.load(Ordering::SeqCst)), wy),
        };
        self.inner.cur_x.store(x, Ordering::SeqCst);
        self.inner.cur_y.store(y, Ordering::SeqCst);
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }

    /// 初始化：按持久化的显示器/边/位置停靠，初始 Hidden 态
    pub fn init(&self, win: &WebviewWindow) {
        let monitors = win.available_monitors().unwrap_or_default();
        let (saved_name, saved_side, saved_x, saved_y) = {
            match self.db.lock().ok() {
                Some(db) => (
                    db.get_setting("dock_monitor"),
                    db.get_setting("dock_side"),
                    db.get_setting("dock_x").and_then(|s| s.parse::<i32>().ok()),
                    db.get_setting("dock_y").and_then(|s| s.parse::<i32>().ok()),
                ),
                None => (None, None, None, None),
            }
        };

        let monitor = saved_name
            .as_deref()
            .and_then(|n| {
                monitors
                    .iter()
                    .find(|m| m.name().map(|mn| mn == n).unwrap_or(false))
            })
            .cloned()
            .or_else(|| win.primary_monitor().ok().flatten())
            .or_else(|| monitors.first().cloned());
        let Some(monitor) = monitor else { return };

        // 窗口初建未落到目标屏时 win.scale_factor() 可能失真——以显示器为准
        let scale = monitor.scale_factor();
        let wa = monitor.work_area();
        let (wx, wy, ww, wh) = if wa.size.width > 0 && wa.size.height > 0 {
            (
                wa.position.x,
                wa.position.y,
                wa.size.width as i32,
                wa.size.height as i32,
            )
        } else {
            (
                monitor.position().x,
                monitor.position().y,
                monitor.size().width as i32,
                monitor.size().height as i32,
            )
        };

        self.inner.wa_x.store(wx, Ordering::SeqCst);
        self.inner.wa_y.store(wy, Ordering::SeqCst);
        self.inner.wa_w.store(ww, Ordering::SeqCst);
        self.inner.wa_h.store(wh, Ordering::SeqCst);
        self.inner
            .scale_milli
            .store((scale * 1000.0).round() as u32, Ordering::SeqCst);

        let Ok(size) = win.inner_size() else { return };
        let (w, h) = (size.width as i32, size.height as i32);
        let edge = Edge::parse(saved_side.as_deref().unwrap_or("right"));
        self.inner.edge.store(edge as u8, Ordering::SeqCst);

        let y_default = wy + ((wh - h) / 2).max(0);
        let x_default = wx + ((ww - w) / 2).max(0);
        let (x, y) = match edge {
            Edge::Left => (wx, saved_y.unwrap_or(y_default)),
            Edge::Right => (wx + ww - w, saved_y.unwrap_or(y_default)),
            Edge::Top => (saved_x.unwrap_or(x_default), wy),
        };
        let (x, y) = (
            x.clamp(wx, (wx + ww - w).max(wx)),
            y.clamp(wy, (wy + wh - h).max(wy)),
        );
        let _ = win.set_position(PhysicalPosition::new(x, y));
        self.inner.cur_x.store(x, Ordering::SeqCst);
        self.inner.cur_y.store(y, Ordering::SeqCst);

        let hidden = (STRIP_HIDDEN * scale).round() as i32;
        self.inner.cur_strip.store(hidden, Ordering::SeqCst);
        self.inner.state.store(0, Ordering::SeqCst);
        self.apply_region(win, hidden);
        self.inner.ready.store(true, Ordering::SeqCst);

        if let Some(name) = monitor.name().map(|n| n.to_string()) {
            self.persist(&name, edge, x, y);
        }
    }

    fn settle(&self, app: &AppHandle, win: &WebviewWindow, target: DockState) {
        let scale = self.inner.scale_milli.load(Ordering::SeqCst) as f64 / 1000.0;
        let Ok(size) = win.inner_size() else { return };
        let strip = target.strip_px(scale, self.edge(), size.width as i32, size.height as i32);
        self.inner.cur_strip.store(strip, Ordering::SeqCst);
        self.inner
            .state
            .store(match target {
                DockState::Hidden => 0,
                DockState::Handle => 1,
                DockState::Panel => 2,
            }, Ordering::SeqCst);
        self.apply_region(win, strip);
        self.inner.animating.store(false, Ordering::SeqCst);
        let _ = app.emit("dock-changed", target.as_str());
    }

    /// 切换到目标状态：先重申贴边几何，再弹性动画窄条（窗口不漂移）
    pub fn dock_to(&self, app: &AppHandle, target: DockState) {
        if !self.inner.ready.load(Ordering::SeqCst) {
            return;
        }
        let Some(win) = app.get_webview_window("main") else { return };
        self.assert_flush(&win);

        let scale = self.inner.scale_milli.load(Ordering::SeqCst) as f64 / 1000.0;
        let Ok(size) = win.inner_size() else { return };
        let from = self.inner.cur_strip.load(Ordering::SeqCst);
        let to = target.strip_px(scale, self.edge(), size.width as i32, size.height as i32);
        // 动画开始即广播目标态：前端 CSS 内容滑动与区域裁剪同步起跑
        let _ = app.emit("dock-anim", target.as_str());
        if from == to {
            self.settle(app, &win, target);
            return;
        }

        let gen = self.inner.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.inner.animating.store(true, Ordering::SeqCst);
        let inner = self.inner.clone();
        let win2 = win.clone();
        let app2 = app.clone();
        std::thread::spawn(move || {
            let total_ms = 260u32;
            let start = std::time::Instant::now();
            let expanding = to > from;
            loop {
                if inner.gen.load(Ordering::SeqCst) != gen {
                    return; // 被新动画取代
                }
                let t = start.elapsed().as_millis() as f64 / total_ms as f64;
                if t >= 1.0 {
                    break;
                }
                let eased = if expanding { ease_out_back(t) } else { ease_in_cubic(t) };
                let w = (from as f64 + (to - from) as f64 * eased).round() as i32;
                inner.cur_strip.store(w, Ordering::SeqCst);
                apply_region_of(&inner, &win2, w);
                std::thread::sleep(std::time::Duration::from_millis(8));
            }
            if inner.gen.load(Ordering::SeqCst) == gen {
                inner.cur_strip.store(to, Ordering::SeqCst);
                inner.state.store(match target {
                    DockState::Hidden => 0,
                    DockState::Handle => 1,
                    DockState::Panel => 2,
                }, Ordering::SeqCst);
                apply_region_of(&inner, &win2, to);
                inner.animating.store(false, Ordering::SeqCst);
                let _ = app2.emit("dock-changed", target.as_str());
            }
        });
    }

    /// 头部拖动：解除裁剪 → 系统级拖动循环（跨屏/DPI 全交给 Windows）→ 松手吸附
    pub fn begin_drag(&self, app: &AppHandle) {
        if !self.inner.ready.load(Ordering::SeqCst) {
            return;
        }
        let Some(win) = app.get_webview_window("main") else { return };
        let full = self.full_strip(&win);
        self.apply_region(&win, full);
        self.inner.cur_strip.store(full, Ordering::SeqCst);
        let _ = win.start_dragging();
        self.finish_dock(app);
    }

    /// 拖动结束：按最大重叠屏 + 最近边（左/右/上）吸附，并持久化
    pub fn finish_dock(&self, app: &AppHandle) {
        let Some(win) = app.get_webview_window("main") else { return };
        let Ok(pos) = win.outer_position() else { return };
        let Ok(size) = win.inner_size() else { return };
        let monitors = win.available_monitors().unwrap_or_default();

        let (rl, rt, rr, rb) = (
            pos.x,
            pos.y,
            pos.x + size.width as i32,
            pos.y + size.height as i32,
        );
        let monitor = monitors
            .iter()
            .map(|m| {
                let mp = m.position();
                let ms = m.size();
                let (ml, mt, mr, mb) =
                    (mp.x, mp.y, mp.x + ms.width as i32, mp.y + ms.height as i32);
                let ow = (mr.min(rr)).saturating_sub(ml.max(rl));
                let oh = (mb.min(rb)).saturating_sub(mt.max(rt));
                (ow.saturating_mul(oh), m)
            })
            .max_by_key(|(area, _)| *area)
            .map(|(_, m)| m)
            .cloned()
            .or_else(|| win.primary_monitor().ok().flatten());
        let Some(monitor) = monitor else { return };

        let scale = monitor.scale_factor();
        let wa = monitor.work_area();
        let (wx, wy, ww, wh) = if wa.size.width > 0 && wa.size.height > 0 {
            (
                wa.position.x,
                wa.position.y,
                wa.size.width as i32,
                wa.size.height as i32,
            )
        } else {
            (
                monitor.position().x,
                monitor.position().y,
                monitor.size().width as i32,
                monitor.size().height as i32,
            )
        };
        let (wr, wb) = (wx + ww, wy + wh);

        // 最近边：窗口各边到工作区对应边的距离取最小（底部不参与）
        let d_left = (rl - wx).abs();
        let d_right = (wr - rr).abs();
        let d_top = (rt - wy).abs();
        let edge = if d_top <= d_left && d_top <= d_right {
            Edge::Top
        } else if d_left <= d_right {
            Edge::Left
        } else {
            Edge::Right
        };

        let (w, h) = (size.width as i32, size.height as i32);
        let (x, y) = match edge {
            Edge::Left => (wx, pos.y.clamp(wy, (wb - h).max(wy))),
            Edge::Right => (wr - w, pos.y.clamp(wy, (wb - h).max(wy))),
            Edge::Top => (pos.x.clamp(wx, (wr - w).max(wx)), wy),
        };
        let _ = win.set_position(PhysicalPosition::new(x, y));

        self.inner.wa_x.store(wx, Ordering::SeqCst);
        self.inner.wa_y.store(wy, Ordering::SeqCst);
        self.inner.wa_w.store(ww, Ordering::SeqCst);
        self.inner.wa_h.store(wh, Ordering::SeqCst);
        self.inner
            .scale_milli
            .store((scale * 1000.0).round() as u32, Ordering::SeqCst);
        self.inner.edge.store(edge as u8, Ordering::SeqCst);
        self.inner.cur_x.store(x, Ordering::SeqCst);
        self.inner.cur_y.store(y, Ordering::SeqCst);

        // 保持当前状态，按新边重新裁剪
        let state = self.cur_state();
        let strip = state.strip_px(scale, edge, w, h);
        self.inner.cur_strip.store(strip, Ordering::SeqCst);
        self.apply_region(&win, strip);

        if let Some(name) = monitor.name().map(|n| n.to_string()) {
            self.persist(&name, edge, x, y);
        }
        let _ = app.emit("dock-side-changed", edge.as_str());
        let _ = app.emit("dock-changed", state.as_str());
    }

    /// 托盘/菜单切换停靠边：保持另一轴的位置
    pub fn set_edge(&self, app: &AppHandle, edge: Edge) {
        if !self.inner.ready.load(Ordering::SeqCst) {
            return;
        }
        let Some(win) = app.get_webview_window("main") else { return };
        let Ok(size) = win.inner_size() else { return };
        let (wx, wy, ww, wh, _) = self.work_area();
        let (w, h) = (size.width as i32, size.height as i32);
        let (x, y) = match edge {
            Edge::Left => (wx, self.inner.cur_y.load(Ordering::SeqCst).clamp(wy, (wy + wh - h).max(wy))),
            Edge::Right => (wx + ww - w, self.inner.cur_y.load(Ordering::SeqCst).clamp(wy, (wy + wh - h).max(wy))),
            Edge::Top => (self.inner.cur_x.load(Ordering::SeqCst).clamp(wx, (wx + ww - w).max(wx)), wy),
        };
        let _ = win.set_position(PhysicalPosition::new(x, y));

        self.inner.edge.store(edge as u8, Ordering::SeqCst);
        self.inner.cur_x.store(x, Ordering::SeqCst);
        self.inner.cur_y.store(y, Ordering::SeqCst);

        let scale = self.inner.scale_milli.load(Ordering::SeqCst) as f64 / 1000.0;
        let state = self.cur_state();
        let strip = state.strip_px(scale, edge, w, h);
        self.inner.cur_strip.store(strip, Ordering::SeqCst);
        self.apply_region(&win, strip);

        let name = self
            .db
            .lock()
            .ok()
            .and_then(|db| db.get_setting("dock_monitor"))
            .unwrap_or_default();
        self.persist(&name, edge, x, y);
        let _ = app.emit("dock-side-changed", edge.as_str());
    }
}

fn ease_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn ease_in_cubic(t: f64) -> f64 {
    t * t * t
}
