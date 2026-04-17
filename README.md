# FeatherMD

一款 Windows 平台的轻量级 Markdown 查看器。

## 特性

- 极速启动（< 500ms）、低内存占用（< 40MB）
- 支持 Mermaid 图表、多主题切换
- 文件修改后自动刷新
- 支持双击 .md 文件直接打开

## 快速开始

```bash
# 构建
cargo build --release

# 运行
feather-md.exe <file.md>

# 注册文件关联
feather-md.exe --register
```

## 技术栈

Rust + WebView2 + marked.js + highlight.js + mermaid.js