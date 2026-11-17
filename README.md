# 边芽 EdgeSprout

常驻屏幕边缘的暗色玻璃 HUD 任务清单——平时一条微光贴边，鼠标靠近弹性展开异形面板。

> **状态**：v0.1.0 首版完成（2026-11-16）。安装包 `src-tauri/target/release/bundle/nsis/EdgeSprout_0.1.0_x64-setup.exe`（2.25 MiB），运行内存 ~31 MB。
> 已验证：三态贴边/五种任务渲染/分组/倒计时徽章/拖延与过期标记/把手计数/托盘。多显示器当前锁定主屏右缘。

## 功能

- **五种任务**：一次性（截止日期/时刻）· 每日 · 每周（周几多选）· 周期（每 N 天）· 流水记录（不定期多次记一笔）
- **三态贴边**：微光竖线（隐藏）→ 把手（日期/剩余数/警示呼吸）→ 完整面板；离开 4s 自动收起，图钉锁定常驻
- **自由停靠**（v4）：左缘/右缘/**上缘**三向；拖动面板头到任意屏幕的边缘松手即吸附最近的边；托盘菜单直达；显示器/边/位置持久化，重启恢复
- **右键菜单**：任意任务右键 → 记一笔（流水）/延到明天/置顶/编辑/删除（带撤销）
- **多屏安全**：窗口矩形永远完整留在停靠屏内，可见性用 Win32 `SetWindowRgn` 裁剪——不越过屏幕边界，不会"漂移"到相邻显示器；每次收起动画前重申贴边几何
- **划掉笔划动画**、延期（一次性顺延/重复跳过当日）、置顶、删除撤销
- **到期提醒**：提前 30 分钟系统通知 + 把手呼吸灯（1h 内到期或过期）
- **快速添加语法**：`!` 置顶 · `@明天` `@后天` `@周五` `@12-01` 日期 · `@21:30` 时刻
- 拖动面板头部可上下移动；托盘菜单（展开/隐藏/退出）

## 技术栈

Tauri 2（Rust）+ SolidJS + 手写 CSS + rusqlite（本地 SQLite，`%APPDATA%/com.edgesprout.app/edge.db`）
窗口永不 resize——三态只移动 x 坐标，展开即整窗从屏幕外滑入，无白闪。

## 开发与打包

需要：Node ≥ 20、Rust（MSVC）、VS Build Tools（C++ 工作负载 + Windows SDK）。

```bat
scripts\dev.cmd     :: 开发调试（自动进入 VS 环境）
scripts\build.cmd   :: 打 NSIS 安装包 → src-tauri\target\release\bundle\nsis\
```

Git Bash 下手动构建：`scripts/cargo-vc.cmd check|build`。

## 设计定调

工业实用 × 复古未来：等宽数字（JetBrains Mono）+ Chakra Petch 展示字、斜切角玻璃面板、
霓虹青 #00E5CC 强调、警示红 #FF4D6A 到期态。记忆点：划掉任务的霓虹笔划 + 边缘呼吸微光。

## 目录

```
src/               SolidJS 前端（组件/状态/样式令牌）
src-tauri/         Rust 侧：dock(贴边) db model remind commands
scripts/           dev/build 批处理 + 图标生成
```
