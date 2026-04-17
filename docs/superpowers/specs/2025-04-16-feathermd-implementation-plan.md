# FeatherMD 实现计划

> 基于 `2025-04-16-feathermd-design.md` 设计文档

---

## 阶段 1：项目骨架与基本窗口

**目标：** 能启动一个 WebView2 窗口并显示静态内容

| 步骤 | 任务 | 产出 |
|------|------|------|
| 1.1 | 初始化 Rust 项目，配置 Cargo.toml 依赖 | `Cargo.toml` |
| 1.2 | 创建内嵌前端资源（index.html + 基础 CSS） | `frontend/` |
| 1.3 | 实现基本 wry 窗口，加载内嵌 HTML | `src/main.rs`, `src/window.rs` |
| 1.4 | 实现 `rust-embed` 内嵌资源加载 | `src/resources.rs` |

**验证点：** 运行 `cargo run`，弹出窗口显示 "FeatherMD" 标题和基础页面

---

## 阶段 2：Markdown 渲染

**目标：** 能通过命令行传入 .md 文件并正确渲染

| 步骤 | 任务 | 产出 |
|------|------|------|
| 2.1 | 集成 marked.js，实现 Markdown 解析渲染 | `frontend/app.js` |
| 2.2 | 集成 highlight.js，实现代码语法高亮 | `frontend/app.js` |
| 2.3 | 实现 Rust 读取 .md 文件内容并传给前端 | `src/app.rs` |
| 2.4 | 实现 CLI 参数解析，支持 `FeatherMD.exe <file>` | `src/main.rs` |
| 2.5 | 窗口标题显示当前文件名 | `src/window.rs` |

**验证点：** `FeatherMD.exe README.md` → 窗口正确渲染 Markdown 内容，代码块有语法高亮

---

## 阶段 3：Mermaid 图表 + 本地图片

**目标：** 支持 Mermaid 图表渲染和本地图片显示

| 步骤 | 任务 | 产出 |
|------|------|------|
| 3.1 | 集成 mermaid.js，检测 mermaid 代码块并渲染 | `frontend/app.js` |
| 3.2 | Mermaid 渲染失败时回退显示原始代码 | `frontend/app.js` |
| 3.3 | 实现本地图片路径解析（相对路径 → file:/// 绝对路径） | `frontend/app.js` |
| 3.4 | 配置 WebView2 虚拟主机映射，允许加载本地文件 | `src/window.rs` |

**验证点：** 包含 Mermaid 图表和本地图片的 .md 文件均能正确渲染

---

## 阶段 4：主题系统

**目标：** 支持多主题切换并持久化偏好

| 步骤 | 任务 | 产出 |
|------|------|------|
| 4.1 | 创建 4 套主题 CSS（github-light/dark, one-dark, nord） | `frontend/themes/` |
| 4.2 | 实现前端主题切换逻辑 | `frontend/app.js` |
| 4.3 | 实现配置文件读写（%APPDATA%/FeatherMD/config.json） | `src/config.rs` |
| 4.4 | 主题切换时保存偏好到配置 | `src/app.rs` ↔ 前端通信 |

**验证点：** 切换主题后重启应用，仍保持上次选择的主题

---

## 阶段 5：文件监听与实时刷新

**目标：** 文件修改后自动刷新内容

| 步骤 | 任务 | 产出 |
|------|------|------|
| 5.1 | 集成 notify crate，监听当前打开文件的变化 | `src/file_watcher.rs` |
| 5.2 | 实现 200ms 防抖逻辑 | `src/file_watcher.rs` |
| 5.3 | 文件变化时通过 PostWebMessage 通知前端刷新 | `src/app.rs` |
| 5.4 | 前端接收通知后重新渲染 | `frontend/app.js` |

**验证点：** 用编辑器修改 .md 文件并保存，查看器自动刷新内容

---

## 阶段 6：文件关联与命令行

**目标：** 双击 .md 文件直接打开

| 步骤 | 任务 | 产出 |
|------|------|------|
| 6.1 | 实现 Windows 注册表文件关联 | `src/file_association.rs` |
| 6.2 | 首次运行自动注册关联，备份已有关联 | `src/file_association.rs` |
| 6.3 | 实现 `--register` / `--unregister` 命令行参数 | `src/main.rs` |
| 6.4 | 实现 `--theme` / `--version` 等其他 CLI 参数 | `src/main.rs` |

**验证点：** 双击 .md 文件 → 自动用 FeatherMD 打开；`--unregister` 后恢复原关联

---

## 阶段 7：交互完善

**目标：** 快捷键、右键菜单、窗口记忆

| 步骤 | 任务 | 产出 |
|------|------|------|
| 7.1 | 实现快捷键（Ctrl+O/T/P, F5, 缩放等） | `frontend/app.js` |
| 7.2 | 实现右键菜单 | `frontend/app.js` |
| 7.3 | 实现窗口位置/大小记忆 | `src/config.rs` |
| 7.4 | 实现 "打开所在文件夹" 和 "用编辑器打开" | `frontend/app.js` ↔ Rust |

**验证点：** 所有快捷键和右键菜单项均可用；重启后窗口位置恢复

---

## 阶段 8：错误处理与收尾

**目标：** 健壮的错误处理 + 发布优化

| 步骤 | 任务 | 产出 |
|------|------|------|
| 8.1 | 文件不存在的友好错误页面 | `frontend/app.js` |
| 8.2 | 文件编码自动检测（UTF-8 / GBK） | `src/app.rs` |
| 8.3 | WebView2 缺失时的提示 | `src/main.rs` |
| 8.4 | Release 编译优化（strip, LTO, 体积压缩） | `Cargo.toml` |
| 8.5 | 应用图标 | `assets/icon.ico` |
| 8.6 | 端到端测试 | 手动测试清单 |

**验证点：** 最终 exe 体积 < 5MB；所有错误场景均有友好提示

---

## 里程碑总结

| 阶段 | 核心交付 | 预估工作量 |
|------|---------|-----------|
| 1 | 项目骨架 + 基本窗口 | 中 |
| 2 | Markdown 渲染 + 代码高亮 | 中 |
| 3 | Mermaid + 本地图片 | 中 |
| 4 | 多主题系统 | 低 |
| 5 | 实时刷新 | 低 |
| 6 | 文件关联 | 中 |
| 7 | 交互完善 | 中 |
| 8 | 错误处理 + 收尾 | 低 |
