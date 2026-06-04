# AI Proxy

本地 AI/LLM 代理服务，统一管理多个大模型供应商的 API 调用。基于 Tauri 2 桌面应用（Rust 后端 + Vue 3 前端），开箱即用。

**仓库地址：** [GitHub](https://github.com/mrhuangyong/ai-proxy) | [Gitee](https://gitee.com/mrhy/ai-proxy)

## 功能特性

### 核心代理

- **多格式代理** — 支持 OpenAI Completions / Responses、Anthropic Messages、Google Gemini 四种格式，自动双向转换
- **流式支持** — 完整的 SSE 流式代理，支持 Thinking/Reasoning、Tool Calls、Function Calling
- **IR 转换层** — 中间表示（IR）架构，任意格式 → IR → 任意格式，新增供应商只需实现 Parser + Generator

### 供应商管理

- **多供应商配置** — 支持配置多个供应商（OpenAI、DeepSeek、Anthropic、Google、自定义等），每个供应商支持独立 Base URL
- **API Key 轮换** — AES-256-GCM 加密存储，最少使用策略自动轮换
- **模型路由** — 根据模型名自动路由到对应供应商，支持跨供应商模型映射
- **模型测试** — 通过管理 API 可直接测试模型连通性

### 拦截与规则

- **拦截规则引擎** — 可配置的请求/响应拦截，支持模型替换、参数覆盖、注入系统提示、响应过滤等
- **条件匹配** — 支持模型名匹配、路径包含、Header 存在、无条件触发等多种条件类型

### MCP 服务器管理

- **MCP 服务器配置** — 统一管理 MCP（Model Context Protocol）服务器，支持 stdio、SSE、Streamable HTTP 三种传输方式
- **应用绑定** — MCP 服务器可绑定到指定应用（Codex CLI/Desktop、Claude CLI/Desktop），按应用启用/禁用
- **导入/同步** — 支持从已安装应用（Claude Desktop、Codex 等）导入 MCP 配置，也可将配置同步写入应用配置文件

### Skill 管理

- **多来源管理** — 支持多个 Skill 来源目录，可设置全局/默认来源和发现顺序
- **发现与扫描** — 自动扫描来源目录下的 Skill，支持从 URL 安装、从市场安装
- **SKILL.md 编辑** — 内置 Skill 描述文件编辑器，支持安装/卸载 Skill

### 可观测性

- **请求日志** — 记录每次请求的 token 用量、延迟、首 token 时间（TTFT）、缓存命中
- **运行日志** — 实时查看代理服务运行日志，支持 WebSocket 实时推送
- **用量统计** — 按模型统计 token 用量和成本估算，ECharts 可视化图表，支持用量趋势分析

### 应用管理

- **应用配置** — 一键生成 Codex CLI、Codex Desktop、Claude CLI、Claude Desktop 等客户端的代理配置
- **模型选择** — 为每个应用独立配置模型（包括 Haiku/Sonnet/Opus 分型号）
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
| `GET /v1/models/{model}` | 单个模型信息 |
| `GET /v1beta/models` | Gemini 模型列表 |
| `GET /health` | 健康检查 |

支持流式（SSE）和非流式两种模式。所有格式之间可以自由转换——例如用 Anthropic 格式请求，实际调用 OpenAI 后端。

## 管理 API

代理服务同时提供 RESTful 管理 API（路径前缀 `/api`）：

| 路径 | 方法 | 说明 |
|------|------|------|
| `/api/providers` | GET/POST | 供应商列表 / 创建供应商 |
| `/api/providers/:id` | PUT/DELETE | 更新 / 删除供应商 |
| `/api/models/test` | POST | 测试模型连通性 |
| `/api/rules` | GET/POST | 拦截规则列表 / 创建规则 |
| `/api/rules/:id` | PUT/DELETE | 更新 / 删除规则 |
| `/api/logs` | GET/DELETE | 请求日志列表 / 清空日志 |
| `/api/logs/:id` | GET | 单条日志详情 |
| `/api/usage` | GET/DELETE | 用量统计 / 清空用量 |
| `/api/usage/trend` | GET | 用量趋势数据 |
| `/api/settings` | GET/PUT | 应用设置 |
| `/api/runtime-logs` | GET | 运行日志 |
| `/api/runtime-logs/stream` | GET (WebSocket) | 运行日志实时推送 |
| `/api/apps` | GET | 应用列表 (仅桌面版) |
| `/api/apps/launch` | POST | 启动应用 (仅桌面版) |
| `/api/apps/:app_type/path` | PUT | 设置应用路径 (仅桌面版) |
| `/api/mcp/servers` | GET/POST | MCP 服务器列表 / 创建 |
| `/api/mcp/servers/:id` | PUT/DELETE | 更新 / 删除 MCP 服务器 |
| `/api/mcp/servers/:id/bindings` | PUT | 更新 MCP 应用绑定 |
| `/api/mcp/import/:app_type` | POST | 从应用导入 MCP 配置 |
| `/api/mcp/apply/:app_type` | POST | 同步 MCP 配置到应用 |
| `/api/skills/sources` | GET/POST | Skill 来源列表 / 创建 |
| `/api/skills/sources/:id` | PUT/DELETE | 更新 / 删除 Skill 来源 |
| `/api/skills` | GET/POST | Skill 列表 / 创建 |
| `/api/skills/:id` | GET/DELETE | Skill 详情 / 删除 |
| `/api/skills/discover` | POST | 发现 Skill |
| `/api/skills/scan` | POST | 扫描 Skill |
| `/api/skills/install` | POST | 安装 Skill |
| `/api/skills/uninstall` | POST | 卸载 Skill |
| `/api/skills/install-from-url` | POST | 从 URL 安装 Skill |
| `/api/skills/install-from-marketplace` | POST | 从市场安装 Skill |
| `/api/skills/:id/skill-md` | PUT | 更新 SKILL.md |
| `/api/skills-marketplace/search` | GET | 搜索 Skill 市场 |

## 项目结构

```
src/                        # Vue 3 前端
  views/
    Dashboard.vue           # 仪表盘
    Providers.vue           # 供应商管理
    Rules.vue               # 拦截规则
    Logs.vue                # 请求日志
    RuntimeLogs.vue         # 运行日志
    Statistics.vue          # 用量统计
    Apps.vue                # 应用管理
    McpServers.vue          # MCP 服务器管理
    Skills.vue              # Skill 管理
    Settings.vue            # 设置
  api/index.ts              # API 客户端（自动发现代理地址）
  types/                    # TypeScript 类型定义
    index.ts                # 核心类型（Provider、Rule、Log 等）
    mcp.ts                  # MCP 相关类型
    skill.ts                # Skill 相关类型
  stores/providers.ts       # Pinia 状态管理
  components/
    UpdateNotification.vue  # 自动更新通知与下载

src-tauri/                  # Rust 后端
  src/
    lib.rs                  # 库入口，模块注册，cfg(desktop/server) 特性门控
    main.rs                 # 桌面版入口
    server_main.rs          # 服务版入口（占位）
    error.rs                # ProxyError 统一错误类型
    http.rs                 # 共享 reqwest::Client
    server/
      router.rs             # 路由定义
      handlers.rs           # 代理转发核心逻辑（含流式 SSE 处理）
      api.rs                # 管理 REST API
      middleware.rs          # CORS + 认证中间件
    converter/              # 多格式转换（IR 层）
      ir.rs                 # 中间表示数据结构（IrRequest、IrResponse、IrStreamChunk 等）
      parsers/              # Completions / Responses / Anthropic / Gemini → IR
      generators/           # IR → Completions / Responses / Anthropic / Gemini
    provider/               # 供应商管理
      endpoint.rs           # Provider、ProviderModel、ApiKeyInfo 类型
      manager.rs            # 模型路由解析（ResolvedRoute）
    routing/                # 请求路由策略
    key/                    # API Key 加密存储（AES-256-GCM）与轮换
    interceptor/            # 拦截规则引擎（条件匹配 + 动作执行）
    mcp/                    # MCP 服务器管理 + 应用配置同步
    skill/                  # Skill 来源管理、扫描、安装
    apps/                   # 应用管理与配置生成（Codex CLI/Desktop、Claude CLI/Desktop）
    auth/                   # 服务版 JWT 认证（用户登录、Token 验证）
    usage/                  # 用量追踪与成本估算
    logging/                # 请求日志 + 运行日志广播层
    db/                     # SQLite 连接池（WAL 模式）+ 编号迁移
    update.rs               # GitHub Release 更新检测与自动下载
    update_timer.rs         # 定时检查更新
  migrations/               # SQLite 迁移脚本（001–013）
```

## 构建模式

项目支持两种 Cargo Feature 构建模式：

| 模式 | Feature | 说明 |
|------|---------|------|
| 桌面版 | `desktop`（默认） | Tauri GUI、系统托盘、自动更新、应用管理 |
| 服务版 | `server` | 无头 CLI 服务，JWT 认证，无 Tauri 依赖 |

```bash
# 桌面版（默认）
pnpm tauri build

# 服务版
cd src-tauri && cargo build --features server --bin ai-proxy-server
```

## GitHub Actions 自动打包

仓库已配置 GitHub Actions，推送版本 tag 或合并到 main 分支时自动构建桌面应用并创建 Release。

### 触发方式

```bash
bash scripts/bump-version.sh <version>   # 自动升级版本号（package.json、tauri.conf.json、Cargo.toml）、创建 tag 并推送
```

或手动：

```bash
git tag v0.5.0
git push origin v0.5.0
```

### 构建产物

- `AI.Proxy_{version}_aarch64.dmg` — macOS Apple Silicon
- `AI.Proxy_{version}_x64-setup.exe` — Windows 安装包
- `AI.Proxy_{version}_x64_en-US.msi` — Windows MSI

## License

[Apache-2.0](LICENSE)
