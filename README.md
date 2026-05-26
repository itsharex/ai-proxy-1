# AI Proxy

本地 AI/LLM 代理服务，统一管理多个大模型供应商的 API 调用。

## 功能

- **多格式代理** — 支持 OpenAI Completions / Responses、Anthropic Messages、Google Gemini 格式，自动转换
- **供应商管理** — 配置多个供应商，每个供应商支持多个 API Key 和模型
- **模型路由** — 根据模型名自动路由到对应供应商
- **API Key 轮换** — AES-256-GCM 加密存储，最少使用策略自动轮换
- **拦截规则** — 可配置的请求/响应拦截引擎，支持模型替换、参数覆盖、注入系统提示等
- **请求日志** — 记录每次请求的 token 用量、延迟、首 token 时间
- **用量统计** — 按模型统计 token 用量和成本估算，可视化图表
- **系统托盘** — 关闭窗口最小化到托盘，托盘菜单控制代理启停

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

## GitHub Actions 自动打包

仓库已配置 GitHub Actions，可在推送版本 tag 时自动打包桌面应用。

### 触发方式

```bash
git tag v0.1.0
git push origin v0.1.0
```

也可以在 GitHub 的 Actions 页面手动触发 `Build Desktop App` workflow。

### 当前产物

- macOS 构建产物
- Windows 构建产物

构建完成后，可在对应 workflow run 的 `Artifacts` 中下载打包结果。

### 当前限制

- 暂不包含 macOS 签名与公证
- 暂不包含 Windows 签名
- 暂不自动创建 GitHub Release
- 暂未包含 Linux 构建

## 项目结构

```
src/                        # Vue 3 前端
  views/                    # 页面组件
    Dashboard.vue           # 仪表盘
    Providers.vue           # 供应商管理
    Models.vue              # 模型总览
    Rules.vue               # 拦截规则
    Apps.vue                # 应用管理
    Logs.vue                # 请求日志
    Statistics.vue          # 用量统计
    Settings.vue            # 设置
  api/index.ts              # API 客户端
  types/index.ts            # 类型定义

src-tauri/                  # Rust 后端
  src/
    lib.rs                  # 应用入口，托盘，代理生命周期
    server/
      handlers.rs           # 代理转发核心逻辑
      router.rs             # 路由定义
      api.rs                # 管理 REST API
      middleware.rs          # CORS + 认证中间件
    converter/              # 多格式转换（IR 层）
      parsers/              # Completions / Responses / Anthropic / Gemini
      generators/           # Completions / Responses / Anthropic / Gemini
    provider/               # 供应商与模型路由
    key/                    # API Key 加密存储与轮换
    interceptor/            # 拦截规则引擎
    apps/                   # 应用管理与启动器
    usage/                  # 用量追踪与成本估算
    logging/                # 请求日志
    db/                     # 数据库初始化与连接池
  migrations/               # SQLite 迁移脚本
```

## 代理端点

客户端只需指向本地代理地址（默认 `http://127.0.0.1:7860`），支持以下格式：

| 端点 | 格式 |
|------|------|
| `POST /v1/chat/completions` | OpenAI Chat Completions |
| `POST /v1/responses` | OpenAI Responses API |
| `POST /v1/messages` | Anthropic Messages |
| `POST /v1beta/models/{model}:generateContent` | Google Gemini |

支持流式（SSE）和非流式两种模式。
