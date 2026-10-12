# QuickClock Rust 版设计文档

日期:2026-10-12
状态:已与需求方确认方案(A:winit + softbuffer + ab_glyph)

## 1. 背景与目标

现有 Python/pygame 版功能完善,但受限于 Python 运行时:单文件版启动需解压、onedir 版需携带 28 MB 依赖、打包链路复杂。Rust 版目标:**一个原生单文件 exe,毫秒级启动,零依赖分发**。

## 2. 功能需求(与 Python 版对齐)

- F1 全屏深灰背景 (#1E1E1E),中央青色 (#00E6E6) 大号时间 HH:MM:SS
- F2 时间字体 Consolas 150pt 等比,日期/星期用微软雅黑
- F3 底部居中:日期(DD/MM/YYYY,30pt),其上方星期英文全称(20pt)
- F4 时间显示本地时区(东八区正确)
- F5 ESC 退出
- F6 单实例:重复启动时唤起已有窗口(还原+置前)后退出

## 3. 非功能需求

- N1 启动到窗口可见 < 100ms(冷启动感知为"瞬间")
- N2 release 产物单文件,体积 < 3 MB
- N3 无控制台窗口(GUI subsystem)
- N4 空闲时 CPU 占用 ~0%(事件驱动,不轮询空转)

## 4. 架构与模块

cargo 项目位于仓库 `rust/` 子目录,与 Python 版并存。

```
rust/
├── Cargo.toml
└── src/
    ├── main.rs      # 入口:单实例守卫 → 初始化 → 事件循环
    ├── single.rs    # Win32 互斥体 + FindWindow 唤起(windows crate)
    ├── timefmt.rs   # 本地时间获取与格式化(chrono,纯函数,可测)
    └── render.rs    # 字体加载/光栅化(ab_glyph)+ softbuffer 绘制
```

## 5. 关键设计决策

- **D1 重绘策略**:winit `ControlFlow::WaitUntil(下一整秒)`,仅在秒变化时重绘,满足 N4。
- **D2 字体加载**:按绝对路径加载系统字体,带回退链:Consolas `consola.ttf` → `lucon.ttf` (Lucida Console) → 加载失败报错退出;雅黑 `msyh.ttc`(TTC 集合取 face 0)→ `msyh.ttf` → `simhei.ttf`(黑体,同为中文字体)。字体属于硬依赖(时钟显示就是文字),全部失败时弹窗报错并退出。
- **D3 单实例**:命名互斥体 `QuickClock_SingleInstance`;窗口标题固定 `QuickClock` 供 `FindWindowW` 定位;已存在则 `ShowWindow(SW_RESTORE)` + `SetForegroundWindow` 后退出,与 Python 版行为一致。
- **D4 DPI**:启用 winit 的 per-monitor DPI 感知,字号按 scale factor 换算,避免高 DPI 屏模糊或过大。
- **D5 布局**:复刻 Python 版版式——时间屏幕居中,星期+日期自底向上堆叠居中;每秒重算布局(窗口尺寸可变,代码不缓存位置)。

## 6. 错误处理

- 首选字体缺失:按 D2 回退链尝试替代字体,继续运行;回退链全部失败:弹窗报错后以非 0 退出码退出。
- softbuffer 初始化失败(无显示设备等):弹窗报错后退出,退出码非 0。
- 单实例唤起时找不到窗口(极端时序):静默退出,与 Python 版一致。

## 7. 测试策略

- `timefmt.rs` 格式化函数:cargo 单元测试(zfill、星期映射、日期格式)。
- 单实例/渲染:手动冒烟(启动→窗口出现→二次启动退出→ESC 关闭),与 Python 版验收流程一致。

## 8. 交付物

- `rust/` cargo 项目,`cargo build --release` 产出 `rust/target/release/QuickClock.exe`
- 构建说明写入 `rust/README.md`
