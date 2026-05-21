# AI LLM Network Proxy - 设计文档

## 概述

一款桌面端 AI LLM 网络代理软件，作为 API 中间层在不同 LLM API 格式之间做转换。支持 OpenAI Completions、OpenAI Responses、Anthropic Messages、Google Gemini 四种格式的互转，包括流式 (SSE) 响应。

**典型场景**: Codex（仅支持 Responses 格式）调用 DeepSeek（仅提供 Completions 格式）时，代理自动将 Responses 请求转换为 Completions 格式转发。

## 技术栈

| 层级 | 选型 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 后端语言 | Rust |
| HTTP Server | axum |
| HTTP Client | reqwest |
| 数据库 | SQLite (sqlx) |
| 序列化 | serde + serde_json |
| 加密 | ring (AES-256-GCM) |
| 前端框架 | Vue 3 + Composition API |
| UI 库 | Naive UI |
| 图表 | ECharts |
| 状态管理 | Pinia |
| 构建 | Vite |

## 架构

### 整体架构

```
┌─────────────────────────────────────────────────────┐
│                   Tauri Desktop App                  │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │          Vue 3 Frontend (WebView)              │  │
│  │  供应商管理 | 模型路由 | 日志调试 | 统计监控 | 设置 │
│  └──────────────────────┬─────────────────────────┘  │
│                         │ Tauri IPC                   │
│  ┌──────────────────────┴─────────────────────────┐  │
│  │              Rust Core (Proxy Engine)           │  │
│  │  HTTP Server | Format Converter | Streaming     │  │
│  │  Key Manager | Router | Storage (SQLite)        │  │
│  │  Interceptor | Usage Tracker                    │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

- Rust 后端运行本地 HTTP 服务器 (axum)，监听 `localhost:PORT`（端口可配置）
- Vue 3 前端通过 Tauri IPC 与后端通信
- 外部客户端将 API endpoint 指向代理地址，代理负责格式转换和转发
- SQLite 存储配置、API Key（加密）、日志、统计数据

### HTTP 绑定配置

- 默认绑定 `127.0.0.1`（仅本地访问）
- 用户可选择绑定 `0.0.0.0`（暴露到网络，供局域网设备使用）
- 支持自定义 IP 绑定
- 选择 `0.0.0.0` 时前端显示安全提示，可选开启 API Key 认证保护代理入口
- HTTP 端口在应用设置中配置，默认 `7860`

### 架构模式: IR 归一化

所有 API 格式先转换为统一的中间表示 (IR)，再从 IR 转为目标格式。扩展新格式只需编写 2 个转换器（解析 + 生成），而非 N-1 个点对点转换器。

```
调用方 → [Responses] → 解析为 IR → [IR] → 转为 Completions → 供应商
```

## IR 中间表示

### 核心结构

```rust
struct IrRequest {
    model: String,
    messages: Vec<IrMessage>,
    tools: Vec<IrTool>,
    tool_choice: Option<IrToolChoice>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    stream: bool,
    response_format: Option<IrResponseFormat>,
    metadata: HashMap<String, Value>,
}

struct IrMessage {
    role: IrRole,
    content: Vec<IrContentPart>,
    tool_call_id: Option<String>,
    tool_calls: Option<Vec<IrToolCall>>,
}

enum IrContentPart {
    Text { text: String },
    Image { url: String, media_type: String },
}

struct IrTool {
    name: String,
    description: String,
    input_schema: Value,
}

struct IrToolCall {
    id: String,
    name: String,
    arguments: String,
}

// 流式响应块
struct IrStreamChunk {
    id: String,                    // 响应 ID
    model: String,
    delta_content: Option<String>, // 增量文本
    delta_tool_calls: Option<Vec<IrToolCallDelta>>,
    finish_reason: Option<String>,
    usage: Option<IrUsage>,
}

struct IrToolCallDelta {
    index: u32,
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

struct IrUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// 完整响应
struct IrResponse {
    id: String,
    model: String,
    choices: Vec<IrChoice>,
    usage: IrUsage,
}

struct IrChoice {
    index: u32,
    message: IrMessage,
    finish_reason: String,
}
```

### 格式差异映射

| 概念 | Completions | Responses | Anthropic | Gemini |
|------|------------|-----------|-----------|--------|
| Tool Calling | `tool_calls[]` | `function_call` | `tool_use` block | `functionCall` |
| System Prompt | system message | `instructions` | `system` 字段 | `systemInstruction` |
| 流式结束 | `data: [DONE]` | `event: response.completed` | `event: message_stop` | 最后 chunk |
| 内容多模态 | `content[]` | `content[]` | `content[]` | `parts[]` |

### 格式支持矩阵

| 格式 | 解析器 (→IR) | 生成器 (IR→) | 流式 |
|------|-------------|-------------|------|
| OpenAI Completions | ✅ | ✅ | SSE |
| OpenAI Responses | ✅ | ✅ | SSE |
| Anthropic Messages | ✅ | ✅ | SSE |
| Google Gemini | ✅ | ✅ | SSE |

## 数据流

```
[Codex/Cursor/等客户端]
        │
        │ POST /v1/responses (OpenAI Responses 格式)
        ▼
┌─── HTTP Server (axum, localhost:PORT) ───┐
│                                           │
│  1. 路由识别 → 匹配请求格式               │
│     /v1/chat/completions  → Completions    │
│     /v1/responses         → Responses      │
│     /v1/messages          → Anthropic      │
│     /v1beta/models/*      → Gemini         │
│                                           │
│  2. 格式解析 → IrRequest                  │
│                                           │
│  3. 前置拦截 (Interceptor Chain)          │
│     规则匹配 → 修改 IR                    │
│                                           │
│  4. 模型路由 (Router)                     │
│     IR.model → 匹配供应商配置             │
│     确定: 目标格式 + Endpoint + API Key   │
│                                           │
│  5. 格式生成 → 目标请求                   │
│                                           │
│  6. 发送请求 (reqwest, 流式)              │
│                                           │
│  7. 流式响应处理                          │
│     供应商 SSE → IrStreamChunk            │
│     IrStreamChunk → 客户端格式 SSE        │
│     实时转发                              │
│                                           │
│  8. 后置处理                              │
│     UsageTracker::record(tokens)          │
│     Logger::log(request, response)        │
│                                           │
└───────────────────────────────────────────┘
        │
        │ SSE stream (客户端格式)
        ▼
[客户端收到流式响应]
```

## 核心功能模块

### 1. 供应商 & API Key 管理

- 供应商列表: OpenAI, Anthropic, Google, DeepSeek, Moonshot, 自定义
- 每个供应商可配置多个 Endpoint:
  - Base URL
  - 支持的格式 (Completions/Responses/Anthropic/Gemini)
  - 认证方式 (Bearer Token, API Key Header)
  - 可用模型列表 (手动或自动拉取)
- API Key 管理:
  - 加密存储: AES-256-GCM，密钥派生自系统 Keychain
  - Key 轮询策略: round-robin / random / least-used
  - Key 状态监控: 有效/过期/限额
  - 多 Key 负载均衡

### 2. 模型路由

- 模型映射表: 将请求中的模型名映射到具体供应商和格式
- 通配符支持: `gpt-*` → 供应商A
- 别名系统: `best-coder` → `claude-sonnet-4-5`
- Fallback 链: 主供应商 → 备选供应商 → 兜底供应商

### 3. 日志 & 调试

- 请求日志: 完整请求/响应记录，格式转换前后对比，Token 用量，延迟追踪
- 调试功能: 请求重放，实时请求监控 (WebSocket 推送到前端)，错误追踪
- 日志存储: SQLite 持久化，可配置保留策略 (按天数/条数)，导出功能 (JSON/CSV)

### 4. 用量统计 & 费用监控

- 按 Token 统计: input/output/total，按模型/供应商/时间段维度
- 费用估算: 内置主流模型价格表，支持自定义价格配置
- 可视化报表: ECharts 图表展示
- 预算告警: 可配置阈值通知

### 5. 请求拦截 & 转换规则

- 前置拦截: 修改请求头、替换模型名称、注入 system prompt、参数覆盖
- 后置拦截: 修改响应内容、过滤敏感信息、响应头注入
- 条件触发: 按模型/来源/路径匹配
- 规则优先级: 可排序，按顺序执行
- 规则格式: 声明式 JSON 配置

## 错误处理

- 客户端错误 (4xx): 格式解析失败 → 400，模型未找到 → 404，认证失败 → 401
- 供应商错误: 转发供应商原始错误，触发 Fallback，Key 轮询自动切换
- 网络错误: 超时重试 (可配置，指数退避)，连接失败 → 502
- 内部错误: 不暴露堆栈，返回通用 500

## 安全

- API Key 存储: AES-256-GCM 加密，密钥通过系统 Keychain 派生
- 本地默认绑定: `127.0.0.1`
- CORS: 绑定 `127.0.0.1` 时仅允许本地来源；绑定 `0.0.0.0` 时允许所有来源
- 日志脱敏: 可配置是否记录请求体
- API Key 展示: 前端仅显示后 4 位
- 网络暴露模式: 可选开启代理入口 API Key 认证

## 非功能性需求

| 项目 | 目标 |
|------|------|
| 代理延迟增加 | < 5ms (非流式)，流式首字节透传 < 2ms |
| 内存占用 | 空闲 < 50MB |
| 启动时间 | < 3s |
| 并发请求 | 支持 50+ 并发 |
| 日志存储 | SQLite 自动清理，默认保留 30 天 |
| 配置文件 | 自动备份，支持导入导出 |

## 项目结构

```
ai-proxy/
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs              # Tauri 入口
│   │   ├── server/              # HTTP 代理服务器
│   │   │   ├── mod.rs
│   │   │   ├── router.rs        # 路由分发
│   │   │   ├── handlers/        # 各格式请求处理
│   │   │   │   ├── completions.rs
│   │   │   │   ├── responses.rs
│   │   │   │   ├── anthropic.rs
│   │   │   │   └── gemini.rs
│   │   │   └── middleware.rs    # 中间件(CORS/日志)
│   │   ├── converter/           # 格式转换引擎
│   │   │   ├── mod.rs
│   │   │   ├── ir.rs           # IR 定义
│   │   │   ├── parsers/        # → IR 解析器
│   │   │   │   ├── completions.rs
│   │   │   │   ├── responses.rs
│   │   │   │   ├── anthropic.rs
│   │   │   │   └── gemini.rs
│   │   │   └── generators/     # IR → 生成器
│   │   │       ├── completions.rs
│   │   │       ├── responses.rs
│   │   │       ├── anthropic.rs
│   │   │       └── gemini.rs
│   │   ├── provider/            # 供应商管理
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs
│   │   │   └── endpoint.rs
│   │   ├── key/                 # API Key 管理
│   │   │   ├── mod.rs
│   │   │   ├── store.rs
│   │   │   └── rotation.rs
│   │   ├── routing/             # 模型路由
│   │   │   ├── mod.rs
│   │   │   ├── router.rs
│   │   │   └── alias.rs
│   │   ├── interceptor/         # 拦截规则引擎
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs
│   │   │   └── rules.rs
│   │   ├── usage/               # 用量统计
│   │   │   ├── mod.rs
│   │   │   ├── tracker.rs
│   │   │   └── pricing.rs
│   │   ├── logging/             # 日志系统
│   │   │   ├── mod.rs
│   │   │   ├── store.rs
│   │   │   └── replay.rs
│   │   ├── db/                  # 数据库层
│   │   │   ├── mod.rs
│   │   │   ├── init.rs
│   │   │   └── pool.rs
│   │   └── ipc/                 # Tauri IPC 命令
│   │       ├── mod.rs
│   │       ├── provider_cmd.rs
│   │       ├── key_cmd.rs
│   │       ├── routing_cmd.rs
│   │       ├── log_cmd.rs
│   │       ├── usage_cmd.rs
│   │       └── interceptor_cmd.rs
│   ├── migrations/              # SQLite 迁移脚本
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                         # Vue 3 前端
│   ├── App.vue
│   ├── main.ts
│   ├── views/
│   │   ├── Dashboard.vue       # 仪表盘概览
│   │   ├── Providers.vue       # 供应商管理
│   │   ├── Models.vue          # 模型路由配置
│   │   ├── Logs.vue            # 日志调试
│   │   ├── Statistics.vue      # 用量统计
│   │   ├── Rules.vue           # 拦截规则
│   │   └── Settings.vue        # 全局设置
│   ├── components/
│   ├── stores/
│   ├── composables/
│   ├── types/
│   └── styles/
├── package.json
├── vite.config.ts
└── tsconfig.json
```
