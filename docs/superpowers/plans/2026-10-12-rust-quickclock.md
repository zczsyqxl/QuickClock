# QuickClock Rust 版实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Rust 重写 QuickClock:全屏时钟,毫秒级启动,单文件 exe < 3 MB。

**Architecture:** winit 事件驱动窗口(整秒唤醒重绘)+ softbuffer 软件帧缓冲 + ab_glyph 字体光栅化;Win32 API 实现单实例唤起;chrono 取本地时间。项目位于仓库 `rust/` 子目录,分支 `rust` 全新历史。

**Tech Stack:** Rust 1.94 (MSVC)、winit 0.30、softbuffer 0.4、ab_glyph 0.2、chrono 0.4、windows-sys 0.59

**Spec:** `docs/superpowers/specs/2026-10-12-rust-quickclock-design.md`

## Global Constraints

- 目标平台 Windows 10+,MSVC 工具链
- `rust/src/main.rs` 顶部 `#![windows_subsystem = "windows"]`(无控制台)
- 颜色常量(softbuffer 像素格式,小端 `0xFFRRGGBB`):背景 `BG=0xFF1E1E1E`,前景 `FG=0xFF00E6E6`
- 窗口标题固定 `QuickClock`;互斥体名 `QuickClock_SingleInstance`
- 字体回退链(spec D2):等宽 `C:\Windows\Fonts\consola.ttf` → `lucon.ttf`;中文 `msyh.ttc`(face 0)→ `msyh.ttf` → `simhei.ttf`
- 版式字号(乘以 DPI scale factor):时间 150、日期 30、星期 20
- 日期格式 `DD/MM/YYYY`,星期为英文全称 Monday..Sunday
- 提交信息用英文;依赖版本锁定在 Cargo.toml,不引入 spec 之外的 crate

## Review Focus

1. **零填充**:单数字时/分/秒必须补零(`00:00:07` 而非 `0:0:7`)→ Task 1 单测覆盖
2. **跨日翻转**:日期/星期必须随当前日期变化,不得缓存(1999-12-31 是 Friday)→ Task 1 用固定日期单测
3. **首选字体缺失**:回退链要能跳过不存在的路径继续找 → Task 3 用"首路径不存在"的列表单测
4. **二次启动时首窗口最小化**:必须先 `ShowWindow(SW_RESTORE)` 再 `SetForegroundWindow` → Task 5 冒烟验证(启动→最小化→再启动)
5. **DPI ≠ 1 的屏**:字号必须乘 scale factor,布局仍居中 → Task 4 对 `center_offset` 与字号缩放单测

---

### Task 1: 时间格式化模块 timefmt

**Files:**
- Create: `rust/Cargo.toml`(cargo new --lib 起步,本任务只需 lib 目标)
- Create: `rust/src/timefmt.rs`
- Modify: `rust/src/lib.rs`(声明 `pub mod timefmt;`)

**Interfaces:**
- Consumes: chrono
- Produces: `pub fn now_local() -> chrono::DateTime<chrono::Local>`;`pub fn format_time(t: &chrono::NaiveTime) -> String`;`pub fn format_date(d: &chrono::NaiveDate) -> String`;`pub fn weekday_name(d: &chrono::NaiveDate) -> &'static str`

- [ ] **Step 1: 初始化 cargo 项目**

`cargo new rust --lib` 后编辑 `rust/Cargo.toml`,dependencies 加 `chrono = "0.4"`;`rust/src/lib.rs` 声明 `pub mod timefmt;`(空文件先建)。

- [ ] **Step 2: 写失败测试**

`rust/src/timefmt.rs` 中 `#[cfg(test)] mod tests`,断言:

```rust
assert_eq!(format_time(&NaiveTime::from_hms_opt(0, 0, 7).unwrap()), "00:00:07");
assert_eq!(format_time(&NaiveTime::from_hms_opt(9, 5, 3).unwrap()), "09:05:03");
assert_eq!(format_time(&NaiveTime::from_hms_opt(23, 59, 59).unwrap()), "23:59:59");
assert_eq!(format_date(&NaiveDate::from_ymd_opt(1999, 12, 31).unwrap()), "31/12/1999");
assert_eq!(weekday_name(&NaiveDate::from_ymd_opt(1999, 12, 31).unwrap()), "Friday");
assert_eq!(weekday_name(&NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()), "Monday");
```

- [ ] **Step 3: 运行确认失败**

Run: `cargo test`(workdir `rust/`)
Expected: 编译失败,`format_time` 等未定义

- [ ] **Step 4: 实现四个函数**

`weekday_name` 用 `d.weekday()` 匹配 chrono `Weekday` 枚举返回 `&'static str`;`format_time`/`format_date` 用 `format!("{:02}")` 风格拼字符串。

- [ ] **Step 5: 运行确认通过并提交**

Run: `cargo test` → PASS
Commit: `git add rust/ && git commit -m "feat: time formatting module with tests"`

### Task 2: 单实例守卫 single

**Files:**
- Create: `rust/src/single.rs`
- Modify: `rust/Cargo.toml`(加 `windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_System_Threading", "Win32_UI_WindowsAndMessaging"] }`)
- Modify: `rust/src/lib.rs`(声明模块)

**Interfaces:**
- Consumes: windows-sys
- Produces: `pub fn acquire_or_raise(mutex_name: &str, window_title: &str) -> bool`(`true` = 本进程是首实例并持有互斥体;`false` = 已有实例,已尝试唤起其窗口,调用方应立即退出)

- [ ] **Step 1: 实现函数**

流程:宽字符编码两参数 → `CreateMutexW` → `GetLastError() == ERROR_ALREADY_EXISTS(183)` 时 `FindWindowW(null, title)`,命中则 `ShowWindow(SW_RESTORE=9)` + `SetForegroundWindow`,返回 `false`;否则返回 `true`。HANDLE 故意 leak(进程生命周期持有,与 Python 版语义一致)。无安全的纯单测路径,正确性由 Task 5 冒烟验证。

- [ ] **Step 2: 编译验证**

Run: `cargo build` → 成功无 warning
Commit: `git add -A rust/ && git commit -m "feat: single-instance guard via Win32 mutex"`

### Task 3: 字体与文字渲染 render

**Files:**
- Create: `rust/src/render.rs`
- Modify: `rust/Cargo.toml`(加 `ab_glyph = "0.2"`)
- Modify: `rust/src/lib.rs`

**Interfaces:**
- Consumes: ab_glyph
- Produces:
  - `pub const BG: u32 = 0xFF1E1E1E;` `pub const FG: u32 = 0xFF00E6E6;`
  - `pub struct Fonts { pub mono: ab_glyph::FontVec, pub cjk: ab_glyph::FontVec }`
  - `pub fn load_font(candidates: &[&str]) -> Option<ab_glyph::FontVec>`(逐个尝试,`FontVec::try_from_vec_and_index(data, 0)`,首个成功者胜出)
  - `pub fn load_fonts() -> Option<Fonts>`(等宽链 + 中文链,任一链全失败返回 None)
  - `pub fn measure(text: &str, font: &FontVec, px: f32) -> (f32, f32)`(宽、高)
  - `pub fn draw_text(pix: &mut [u32], stride: u32, text: &str, font: &FontVec, px: f32, color: u32, x: i32, y: i32)`

- [ ] **Step 1: 写失败测试**

```rust
#[test] fallback_chain_skips_missing_files() {
    // first path does not exist, second does
    let f = load_font(&["Z:\\nope.ttf", "C:\\Windows\\Fonts\\consola.ttf"]);
    assert!(f.is_some());
}
#[test] load_fonts_finds_both_on_windows() { assert!(load_fonts().is_some()); }
#[test] measure_nonzero() {
    let fonts = load_fonts().unwrap();
    assert!(measure("00:00", &fonts.mono, 150.0).0 > 0.0);
    assert!(measure("Monday", &fonts.cjk, 20.0).0 > 0.0);
}
#[test] draw_text_paints_foreground_pixels() {
    // 200x60 buffer filled with BG, draw "0" in FG at (0,0), assert some pixel == FG and buffer dims respected
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test` → 编译失败(类型/函数未定义)

- [ ] **Step 3: 实现**

`measure` 用 `FontVec::glyph_id` + `ScaledFont`/`outline_rect` 累加 advance,高度取 ascent+descent;`draw_text` 对每个字形 `outline_glyph` 后按像素覆盖写入(公式:`pix[y*stride + x] = color`,越界跳过)。测试依赖系统字体存在(本机已确认),属环境绑定测试,可接受。

- [ ] **Step 4: 运行确认通过并提交**

Run: `cargo test` → PASS
Commit: `git add -A rust/ && git commit -m "feat: font loading with fallback chain and text rasterizer"`

### Task 4: 窗口、布局与事件循环 main

**Files:**
- Create: `rust/src/main.rs`
- Modify: `rust/Cargo.toml`(加 `winit = "0.30"`、`softbuffer = "0.4"`,`[[bin]] name = "QuickClock"`,或直接 `src/main.rs` 默认 bin 名改名)
- Modify: `rust/src/lib.rs`(新增 `pub fn center_offset(win_w: i32, text_w: i32) -> i32 { (win_w - text_w) / 2 }`)

**Interfaces:**
- Consumes: Task 1 全部接口、`single::acquire_or_raise`、Task 3 全部接口、winit、softbuffer
- Produces: 可运行的 `QuickClock` 窗口程序

- [ ] **Step 1: 写 center_offset 与字号缩放单测(lib.rs)**

```rust
assert_eq!(center_offset(1000, 300), 350);
assert_eq!(center_offset(100, 101), 0); // 不允许负数
// 字号缩放断言: 150pt @ scale 1.5 → 225.0 px (以纯函数 scaled(px: f32, s: f64) -> f32 驱动)
```

Run: `cargo test` 失败 → 实现 → 通过。

- [ ] **Step 2: 实现 main.rs**

要点全部锁定:`#![windows_subsystem = "windows"]`;启动即调 `acquire_or_raise("QuickClock_SingleInstance", "QuickClock")`,`false` 则 `return`;`load_fonts()` 返回 None 时用 `MessageBoxW`(windows-sys,feature 已含)弹 "Failed to load system fonts" 后 `exit(1)`(spec 第 6 节);winit 事件循环(ApplicationHandler):`Resumed` 建窗口(标题 `QuickClock`、`set_fullscreen(Borderless(None))`)、初始化 softbuffer;`AboutToWait` 中比较当前秒与上次渲染秒,变化则 `request_redraw`,并 `set_wait_until(下一整秒)`;`RedrawRequested` 中填 BG → 用 scale factor 算字号(`scaled(150.0, sf)` 等)→ 按版式绘制(时间屏幕正中,星期/日期自底部居中堆叠,间距 10px*scale)→ present;`KeyboardInput`(Escape)或 `CloseRequested` → exit。

- [ ] **Step 3: 手动冒烟**

Run: `cargo run` → 全屏时钟出现,时间走秒,布局与 Python 版一致;按 ESC 退出
Commit: `git add -A rust/ && git commit -m "feat: fullscreen clock window with event loop"`

### Task 5: 集成验证、README 与收尾

**Files:**
- Create: `rust/README.md`

**Interfaces:**
- Consumes: 前四个任务的完整程序

- [ ] **Step 1: release 构建**

Run: `cargo build --release` → 成功;记录 exe 体积,断言 < 3 MB(`(Get-Item rust/target/release/QuickClock.exe).Length/1MB`)

- [ ] **Step 2: 冒烟清单(PowerShell)**

1. 启动 exe → 窗口标题 `QuickClock` 出现
2. 最小化后再启动一次 → 原窗口被还原置前,新进程立即退出(code=0)
3. ESC → 进程退出
(与 Python 版验收同口径)

- [ ] **Step 3: 写 rust/README.md**

内容:构建命令 `cargo build --release`、产物路径、功能清单、与 Python 版的差异说明。英文书写(仓库规范)。

- [ ] **Step 4: 提交**

Commit: `git add rust/README.md && git commit -m "docs: build instructions for Rust QuickClock"`
