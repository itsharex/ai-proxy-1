# 系统托盘功能设计

## 概述

点击窗口关闭按钮时，应用隐藏到系统托盘而非退出。托盘提供右键菜单，支持恢复窗口、控制代理和退出应用。

## 方案选择

使用 Tauri 2 官方插件 `tauri-plugin-tray`。

## 依赖和配置

### Cargo.toml

添加 `tauri-plugin-tray` 依赖。

### tauri.conf.json

添加 `trayIcon` 配置：图标使用 `icons/32x32.png`，tooltip 显示 "AI Proxy"。

### capabilities/default.json

添加 `tray:default` 权限。

## 窗口关闭拦截

在 `lib.rs` 的 `setup` 闭包中监听主窗口的 `CloseRequested` 事件：

- 阻止默认关闭行为
- 调用 `window.hide()` 隐藏窗口
- 应用继续运行，代理服务不中断

前端无需改动。

## 托盘菜单

菜单项：

| 菜单项 | 行为 |
|--------|------|
| 显示窗口 | `window.show()` + `window.set_focus()` |
| 代理状态 | 只读，显示端口和运行状态（如 "Proxy :7860 running"） |
| 启动/停止代理 | 切换代理服务运行状态 |
| 退出 | `app.exit(0)` 完全退出 |

图标使用 `src-tauri/icons/32x32.png`，静态不变。

## 实现范围

涉及文件：

- `src-tauri/Cargo.toml` — 添加依赖
- `src-tauri/tauri.conf.json` — 添加托盘配置
- `src-tauri/capabilities/default.json` — 添加权限
- `src-tauri/src/lib.rs` — 注册插件、构建菜单、事件处理、窗口拦截
- `src-tauri/src/server/mod.rs` — 可能需要暴露 start/stop 接口

## 不做的事

- 不做动态图标切换
- 不做关闭确认对话框
- 不做设置项让用户选择关闭行为
- 前端无需改动
