# FeatherMD - 极致轻量 Markdown 查看器设计文档

> 日期：2025-04-16
> 状态：已审批

## 概述

FeatherMD 是一款 Windows 平台的极致轻量 Markdown 查看器。核心诉求：双击 .md 文件瞬间打开查看，安装包小，内存占用低。

### 核心目标

| 指标 | 目标值 |
|------|--------|
| 冷启动时间 | < 500ms |
| 文件打开时间 | < 100ms |
| 内存占用（空闲） | < 40MB |
| 安装包体积 | < 5MB（单文件 .exe） |
| CPU 占用（空闲） | < 0.5% |

### 运行环境

- 操作系统：Windows 10 1803+ / Windows 11
- 运行时依赖：WebView2（Win10 2004+ / Win11 已内置）

---

## 技术选型：Rust + WebView2

### 选型理由

1. **Mermaid 是硬需求**——WebView2 允许直接使用 mermaid.js，零成本支持图表渲染
2. **体积极小**——2-5MB 单文件 exe，WebView2 是系统组件不占用安装包体积
3. **WebView2 ≠ Electron**——它是 Windows 系统级组件，不会带来 Electron 的体积膨胀
4. **Web 生态天然支持**——代码高亮、多主题切换、Markdown 解析均有成熟 JS 库

### 核心依赖

| 依赖 | 用途 |
|------|------|
| `wry` | WebView2 的 Rust 封装，窗口管理 |
| `notify` | 文件变化监听 |
| `rust-embed` | 编译时内嵌前端资源 |
| `serde_json` | 配置文件读写 |
| `winreg` | Windows 注册表操作（文件关联） |
| `clap` | 命令行参数解析 |

### 前端依赖

| 依赖 | 用途 | 体积（min+gzip） |
|------|------|-------------------|
| `marked.js` | Markdown 解析 | ~40KB |
| `highlight.js` | 代码语法高亮 | ~30KB（按需加载语言） |
| `mermaid.js` | 图表渲染 | ~500KB |

---

## 整体架构

```
┌─────────────────────────────────────┐
│         FeatherMD.exe (2-5MB)       │
├──────────────┬──────────────────────┤
│   Rust 核心   │   WebView2 渲染层    │
│              │                      │
│ · 文件读取    │  ┌────────────────┐  │
│ · 文件监听    │  │  HTML/CSS/JS   │  │
│ · 窗口管理    │  │  (内嵌资源)     │  │
│ · 文件关联    │  ├────────────────┤  │
│ · 命令行解析  │  │ · marked.js    │  │
│ · 主题管理    │  │ · highlight.js │  │
│ · 配置读写    │  │ · mermaid.js   │  │
│              │  └────────────────┘  │
└──────────────┴──────────────────────┘
```

### 工作流程

1. 用户双击 .md 文件 → Windows 通过文件关联启动 `FeatherMD.exe "path/to/file.md"`
2. Rust 读取文件内容 → 通过 WebView2 的 `PostWebMessage` 传给前端
3. 前端用 marked.js 解析 Markdown → 渲染到页面
4. 前端检测到 Mermaid 代码块 → 调用 mermaid.js 渲染图表
5. Rust 监听文件变化 → 文件修改时自动刷新内容

---

## 前端渲染层

### 内嵌资源结构

```
内嵌资源:
├── index.html              # 主页面骨架
├── markdown.css            # 基础 Markdown 样式
├── themes/
│   ├── github-light.css    # GitHub 亮色主题
│   ├── github-dark.css     # GitHub 暗色主题
│   ├── one-dark.css        # One Dark 主题
│   └── nord.css            # Nord 主题
├── js/
│   ├── marked.min.js       # Markdown 解析
│   ├── highlight.min.js    # 代码高亮
│   └── mermaid.min.js      # 图表渲染
└── custom-styles.css       # 自定义覆盖样式
```

所有前端资源通过 `rust-embed` 在编译时打包进 exe，运行时无需外部文件。

### Markdown 渲染流水线

```
.md 文件内容
    ↓
marked.js 解析
    ↓
检测代码块语言
    ├── 普通代码 → highlight.js 语法高亮
    ├── mermaid → mermaid.js 渲染图表
    └── 其他 → 原样显示
    ↓
应用当前主题 CSS
    ↓
渲染到页面
```

### 主题切换机制

- 主题列表硬编码在前端
- 用户通过快捷键 `Ctrl+T` 或右键菜单切换
- 当前主题偏好保存在 `%APPDATA%/FeatherMD/config.json`
- 切换时仅替换 CSS 变量，无需重新解析 Markdown

### 本地图片处理

- marked.js 的 renderer 自定义图片路径解析
- 将相对路径转换为 `file:///` 协议的绝对路径
- WebView2 通过 `set_virtual_host_name_to_folder_mapping` 开启本地文件访问

---

## Rust 核心功能

### 窗口管理

| 特性 | 说明 |
|------|------|
| 窗口库 | `wry` crate |
| 窗口标题 | 显示文件名，如 `README.md - FeatherMD` |
| 默认尺寸 | 900×700，居中显示 |
| 记住位置 | 保存上次窗口位置和大小到 config.json |
| 多文件 | 每个文件独立窗口 |

### 文件监听（实时刷新）

- 使用 `notify` crate 监听文件变化
- 防抖处理：修改后 200ms 才触发刷新，避免频繁渲染
- 通过 WebView2 `PostWebMessage` 通知前端重新渲染

### 文件关联

注册表写入位置（不需要管理员权限）：

```
HKEY_CURRENT_USER\Software\Classes\.md
  (默认) = FeatherMD.md

HKEY_CURRENT_USER\Software\Classes\FeatherMD.md\shell\open\command
  (默认) = "path\to\FeatherMD.exe" "%1"

HKEY_CURRENT_USER\Software\Classes\FeatherMD.md\DefaultIcon
  (默认) = "path\to\FeatherMD.exe,0"
```

- 首次运行时自动注册
- 备份之前的关联以便卸载时恢复
- 命令行参数 `--unregister` 取消关联

### 命令行接口

```
FeatherMD.exe [选项] [文件路径]

选项:
  <file>          打开指定 Markdown 文件
  --register      注册文件关联
  --unregister    取消文件关联
  --theme <name>  指定主题
  --version       显示版本
  --help          显示帮助
```

### 配置文件

```json
// %APPDATA%/FeatherMD/config.json
{
  "theme": "github-light",
  "window": {
    "width": 900,
    "height": 700,
    "x": 100,
    "y": 100
  },
  "auto_reload": true,
  "file_association_backup": {
    ".md": "VSCode.md"
  }
}
```

---

## 交互设计

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+O` | 打开文件 |
| `Ctrl+T` | 切换主题 |
| `Ctrl+P` | 打印 |
| `Ctrl+Plus/Minus` | 字体缩放 |
| `Ctrl+0` | 重置缩放 |
| `F5` | 手动刷新 |
| `Ctrl+Shift+C` | 复制当前代码块 |
| `Esc` | 退出 |

### 右键菜单

```
┌────────────────────┐
│ 复制               │
│ 复制为 Markdown    │
│ ───────────────── │
│ 切换主题 ►         │
│   GitHub Light     │
│   GitHub Dark  ✓   │
│   One Dark         │
│   Nord             │
│ ───────────────── │
│ 字体放大           │
│ 字体缩小           │
│ 重置缩放           │
│ ───────────────── │
│ 打开所在文件夹      │
│ 用编辑器打开        │
└────────────────────┘
```

---

## 错误处理

| 场景 | 处理方式 |
|------|---------|
| 文件不存在 | 显示友好错误页面，提供"重新选择文件"按钮 |
| 文件编码问题 | 自动检测 UTF-8 / GBK，失败时提示选择编码 |
| Mermaid 渲染失败 | 回退显示原始代码块，不阻塞其他内容 |
| WebView2 缺失 | 提示下载 WebView2 Runtime（仅影响 Win7/早期 Win10） |
| 文件占用/无权限 | 显示错误提示，不崩溃 |

---

## 不做的事情（YAGNI）

以下功能明确**不在**当前范围内：

- ❌ Markdown 编辑功能（这是查看器，不是编辑器）
- ❌ 导出为 PDF / HTML
- ❌ 多标签页（每个文件独立窗口，保持简单）
- ❌ 插件系统
- ❌ 同步/云存储
- ❌ 语法检查 / lint
- ❌ 拼写检查

---

## 项目结构

```
feather-md/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口，命令行解析
│   ├── app.rs               # 应用主逻辑
│   ├── window.rs            # 窗口管理
│   ├── file_watcher.rs      # 文件监听
│   ├── file_association.rs  # 文件关联
│   ├── config.rs            # 配置管理
│   └── resources.rs         # 内嵌资源管理
├── frontend/
│   ├── index.html
│   ├── markdown.css
│   ├── themes/
│   │   ├── github-light.css
│   │   ├── github-dark.css
│   │   ├── one-dark.css
│   │   └── nord.css
│   ├── js/
│   │   ├── marked.min.js
│   │   ├── highlight.min.js
│   │   └── mermaid.min.js
│   ├── app.js               # 前端主逻辑
│   └── custom-styles.css
├── assets/
│   └── icon.ico             # 应用图标
└── docs/
    └── superpowers/
        └── specs/
            └── 2025-04-16-feathermd-design.md
```
