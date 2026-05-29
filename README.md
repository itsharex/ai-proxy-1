# AI Proxy

本地 AI/LLM 代理服务，统一管理多个大模型供应商的 API 调用。基于 Tauri 2 桌面应用（Rust 后端 + Vue 3 前端），开箱即用。

## 功能特性

### 核心代理

- **多格式代理** — 支持 OpenAI Completions / Responses、Anthropic Messages、Google Gemini 四种格式，自动双向转换
- **流式支持** — 完整的 SSE 流式代理，支持 Thinking/Reasoning、Tool Calls、Function Calling
- **IR 转换层** — 中间表示（IR）架构，任意格式 → IR → 任意格式，新增供应商只需实现 Parser + Generator

### 供应商管理

- **多供应商配置** — 支持配置多个供应商（OpenAI、DeepSeek、Anthropic、Google、自定义等），每个供应商支持独立 Base URL
- **API Key 轮换** — AES-256-GCM 加密存储，最少使用策略自动轮换
- **模型路由** — 根据模型名自动路由到对应供应商，支持跨供应商模型映射

### 拦截与规则

- **拦截规则引擎** — 可配置的请求/响应拦截，支持模型替换、参数覆盖、注入系统提示等

### 可观测性

- **请求日志** — 记录每次请求的 token 用量、延迟、首 token 时间（TTFT）
- **运行日志** — 实时查看代理服务运行日志
- **用量统计** — 按模型统计 token 用量和成本估算，ECharts 可视化图表

### 应用管理

- **应用配置** — 一键生成 Codex CLI、Claude Desktop、CherryStudio 等客户端的代理配置
- **自动启动** — 支持配置常用应用的启动参数

### 桌面集成

- **系统托盘** — 关闭窗口最小化到托盘，托盘菜单控制代理启停
- **自动更新** — 检测新版本后自动下载安装包到下载目录，一键安装
- **开机自启** — 支持 macOS LaunchAgent 开机自启动

## 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | Tauri 2 (Rust) |
| 前端 | Vue 3 + TypeScript + Naive UI + ECharts |
| 代理服务 | Axum 0.7 |
| 数据库 | SQLite (SQLx) |
| 构建 | Vite 6 + pnpm |

## 快速开始

### 环境要求

- Rust (edition 2021)
- Node.js 18+
- pnpm

### 开发模式

```bash
pnpm install
pnpm tauri dev
```

### 生产构建

```bash
pnpm tauri build
```

## 代理端点

客户端只需将 API Base URL 指向本地代理地址（默认 `http://127.0.0.1:7860`），支持以下格式：

| 端点 | 格式 |
|------|------|
| `POST /v1/chat/completions` | OpenAI Chat Completions |
| `POST /v1/responses` | OpenAI Responses API |
| `POST /v1/messages` | Anthropic Messages |
| `POST /v1beta/models/{model}:generateContent` | Google Gemini (非流式) |
| `POST /v1beta/models/{model}:streamGenerateContent` | Google Gemini (流式) |
| `GET /v1/models` | 模型列表 |
| `GET /health` | 健康检查 |

支持流式（SSE）和非流式两种模式。所有格式之间可以自由转换——例如用 Anthropic 格式请求，实际调用 OpenAI 后端。

## 项目结构

```
src/                        # Vue 3 前端
  views/
    Dashboard.vue           # 仪表盘
    Providers.vue           # 供应商管理
    Models.vue              # 模型总览
    Rules.vue               # 拦截规则
    Apps.vue                # 应用管理
    Logs.vue                # 请求日志
    RuntimeLogs.vue         # 运行日志
    Statistics.vue          # 用量统计
    Settings.vue            # 设置
  api/index.ts              # API 客户端
  types/index.ts            # 类型定义
  stores/                   # Pinia 状态管理
  components/
    UpdateNotification.vue  # 自动更新通知与下载

src-tauri/                  # Rust 后端
  src/
    lib.rs                  # 应用入口，托盘，代理生命周期
    server/
      handlers.rs           # 代理转发核心逻辑（含流式 SSE 处理）
      router.rs             # 路由定义
      api.rs                # 管理 REST API
      middleware.rs          # CORS + 认证中间件
    converter/              # 多格式转换（IR 层）
      ir.rs                 # 中间表示数据结构
      parsers/              # Completions / Responses / Anthropic / Gemini → IR
      generators/           # IR → Completions / Responses / Anthropic / Gemini
    provider/               # 供应商与模型路由
    routing/                # 请求路由策略
    key/                    # API Key 加密存储与轮换（AES-256-GCM）
    interceptor/            # 拦截规则引擎
    apps/                   # 应用管理与配置生成
    usage/                  # 用量追踪与成本估算
    logging/                # 请求日志
    db/                     # SQLite 数据库初始化与连接池
    update.rs               # GitHub Release 更新检测与自动下载
    update_timer.rs         # 定时检查更新
  migrations/               # SQLite 迁移脚本
```

## GitHub Actions 自动打包

仓库已配置 GitHub Actions，推送版本 tag 时自动构建桌面应用并创建 Release。

### 触发方式

```bash
bash scripts/bump-version.sh  # 自动升级版本号、创建 tag 并推送
```

或手动：

```bash
git tag v0.3.0
git push origin v0.3.0
```

### 构建产物

- `AI.Proxy_{version}_aarch64.dmg` — macOS Apple Silicon
- `AI.Proxy_{version}_x64-setup.exe` — Windows 安装包
- `AI.Proxy_{version}_x64_en-US.msi` — Windows MSI

## License

Private
