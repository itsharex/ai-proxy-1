# 应用管理功能设计

## 概述

在 AI Proxy 中新增"应用管理"功能，允许用户通过本软件直接启动 Codex CLI、Codex Desktop、Claude CLI、Claude Desktop，并自动将代理地址和所选模型写入对应应用的配置文件。

## 支持的应用（固定 4 个）

| 应用 | 配置文件 | 代理字段 | 模型字段 |
|---|---|---|---|
| Codex CLI | `~/.codex/config.toml` | `openai_base_url` | `model` |
| Codex Desktop | `~/.codex/config.toml`（与 CLI 共用） | `openai_base_url` | `model` |
| Claude CLI | `~/.claude/settings.json` | `env.ANTHROPIC_BASE_URL` | `env.ANTHROPIC_MODEL` |
| Claude Desktop | 参考 cc-switch 方式，修改 Claude Desktop 配置文件 | 待确认 | 待确认 |

## 操作流程

1. 用户在"应用管理"页面看到 4 张应用卡片
2. 点击某应用的「打开」按钮
3. 弹出 Modal，显示模型选择下拉框（数据来自现有 provider_models）
4. 用户选择模型，点击「确定」
5. 后端写入代理地址 + 模型到对应配置文件
6. 后端启动应用（detached 进程）
7. 记录启动日志（每个应用仅保留最后一次）
8. 前端提示"已启动"，卡片更新显示

## 数据模型

新增数据库表 `app_configs`，每个应用仅保留一条记录：

```sql
CREATE TABLE IF NOT EXISTS app_configs (
    app_type TEXT PRIMARY KEY,        -- 'codex_cli' | 'codex_desktop' | 'claude_cli' | 'claude_desktop'
    model TEXT NOT NULL,              -- 用户选择的模型名
    proxy_url TEXT NOT NULL,          -- 写入的代理地址
    launched_at TEXT NOT NULL,        -- 最后启动时间（ISO 格式）
    config_path TEXT,                 -- 实际写入的配置文件路径
    install_path TEXT,                -- 应用安装路径（用户手动指定或自动检测）
    status TEXT NOT NULL DEFAULT 'success'  -- 'success' | 'config_error' | 'launch_error'
);
```

使用 `INSERT OR REPLACE` 确保每个 `app_type` 仅保留最新记录。

## 后端架构

新增模块 `src-tauri/src/apps/`：

### 文件结构

| 文件 | 职责 |
|---|---|
| `mod.rs` | 模块入口，导出子模块 |
| `types.rs` | `AppType` 枚举、配置结构体、请求/响应类型 |
| `config.rs` | 各应用配置文件的读写逻辑（TOML/JSON 解析、字段写入） |
| `launcher.rs` | 跨平台应用路径检测 + 进程启动 |
| `handlers.rs` | Axum 路由处理函数 |

### 依赖

- `toml` crate：解析和写入 Codex 的 TOML 配置
- `serde_json`：已有依赖，用于 Claude CLI/Desktop 的 JSON 配置
- `tokio::process::Command`：异步启动进程

### 配置文件写入策略

#### Codex（CLI/Desktop）

配置文件路径：`~/.codex/config.toml`

写入逻辑：
1. 检测文件是否存在，不存在则创建
2. 用 `toml` crate 解析现有内容
3. 更新 `model` 和 `openai_base_url` 字段
4. 原子写入（先写临时文件，再 rename）

示例写入内容：
```toml
model = "glm-5.1"
openai_base_url = "http://127.0.0.1:7860/v1"
```

#### Claude CLI

配置文件路径：`~/.claude/settings.json`

写入逻辑：
1. 读取现有 JSON，保留所有其他字段不变
2. 更新 `env.ANTHROPIC_BASE_URL` 和 `env.ANTHROPIC_MODEL`
3. 原子写入

示例写入内容：
```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:7860",
    "ANTHROPIC_MODEL": "glm-5.1"
  }
}
```

#### Claude Desktop

参考 cc-switch 项目的方式修改 Claude Desktop 配置文件。macOS 路径：`~/Library/Application Support/Claude/claude_desktop_config.json`。具体写入字段待实现时根据 cc-switch 源码确认。

### 应用路径检测

#### macOS / Linux

| 应用 | 检测方式 |
|---|---|
| Codex CLI | `which codex` |
| Codex Desktop | 检查 `/Applications/Codex.app`（macOS） |
| Claude CLI | `which claude` |
| Claude Desktop | 检查 `/Applications/Claude.app`（macOS） |

#### Windows

检查常见安装路径（`Program Files`、`AppData` 等），若检测不到则提示用户手动配置。

#### 路径优先级

1. 用户手动配置的 `install_path`（最高优先级）
2. 自动检测结果
3. 都没有 → 前端显示"未检测到"，引导用户配置

### 进程启动

```rust
// macOS: Desktop 应用
tokio::process::Command::new("open").arg(app_path).spawn()

// Linux: Desktop 应用
tokio::process::Command::new("xdg-open").arg(app_path).spawn()

// Windows: Desktop 应用
tokio::process::Command::new("cmd").args(["/C", "start", "", &app_path]).spawn()

// CLI 应用（所有平台）
tokio::process::Command::new(detected_path).spawn()
```

所有启动均为 detached，不跟踪进程状态。

### API 端点

新增路由挂载在 `/api/apps` 下：

| 端点 | 方法 | 功能 |
|---|---|---|
| `/api/apps` | GET | 返回 4 个应用的状态（已安装、路径、最后启动配置） |
| `/api/apps/launch` | POST | 传入 `{ app_type, model }` → 写配置 + 启动 + 更新数据库 |
| `/api/apps/:app_type/path` | PUT | 手动设置应用安装路径 |

#### GET /api/apps 响应

```json
{
  "success": true,
  "data": [
    {
      "app_type": "codex_cli",
      "installed": true,
      "install_path": "/usr/local/bin/codex",
      "config_path": "/Users/xxx/.codex/config.toml",
      "model": "glm-5.1",
      "proxy_url": "http://127.0.0.1:7860/v1",
      "launched_at": "2026-05-25T10:30:00Z",
      "status": "success"
    },
    {
      "app_type": "claude_desktop",
      "installed": false,
      "install_path": null,
      "config_path": null,
      "model": null,
      "proxy_url": null,
      "launched_at": null,
      "status": null
    }
  ]
}
```

#### POST /api/apps/launch 请求

```json
{
  "app_type": "claude_cli",
  "model": "glm-5.1"
}
```

#### POST /api/apps/launch 响应

```json
{
  "success": true,
  "data": {
    "app_type": "claude_cli",
    "model": "glm-5.1",
    "proxy_url": "http://127.0.0.1:7860",
    "config_path": "/Users/xxx/.claude/settings.json",
    "launched_at": "2026-05-25T10:30:00Z",
    "status": "success"
  }
}
```

#### PUT /api/apps/:app_type/path 请求

```json
{
  "install_path": "C:\\Users\\xxx\\AppData\\Local\\Programs\\claude\\claude.exe"
}
```

## 前端设计

### 新增文件

- `src/views/Apps.vue`：应用管理页面

### 修改文件

- `src/router.ts`：新增 `/apps` 路由
- `src/App.vue`：侧边栏新增"应用管理"菜单项
- `src/types/index.ts`：新增应用相关类型定义

### 页面布局

卡片网格展示 4 个应用，每个应用一张卡片：

```
┌──────────────────┐  ┌──────────────────┐
│  Codex CLI    ⚙️ │  │  Codex Desktop ⚙️│
│  ✅ 已安装        │  │  ✅ 已安装        │
│  路径: /usr/...  │  │  路径: /App...   │
│  模型: gpt-4     │  │  模型: ---        │
│  [打开]          │  │  [打开]          │
└──────────────────┘  └──────────────────┘
┌──────────────────┐  ┌──────────────────┐
│  Claude CLI   ⚙️ │  │  Claude Desktop⚙️│
│  ✅ 已安装        │  │  ✅ 已安装        │
│  路径: /usr/...  │  │  路径: /App...   │
│  模型: ---        │  │  模型: ---        │
│  [打开]          │  │  [打开]          │
└──────────────────┘  └──────────────────┘
```

卡片右上角齿轮图标用于配置安装路径（弹窗包含路径输入框，提示"留空则自动检测"）。

### 交互流程

1. 页面加载时调用 `GET /api/apps` 获取所有应用状态
2. 点击「打开」→ 弹出 Modal
3. Modal 内包含模型选择下拉框（数据来源：现有 provider_models 表）
4. 选择模型 → 点击「确定」→ 调用 `POST /api/apps/launch`
5. 成功后提示"已启动"，更新卡片上的模型信息
6. 失败则显示错误信息

### 侧边栏菜单

在 `App.vue` 的 `menuOptions` 中新增：

```ts
{ label: '应用管理', key: '/apps', icon: renderIcon(AppsIcon) }
```

### 类型定义

```typescript
interface AppConfig {
  app_type: AppType
  installed: boolean
  install_path: string | null
  config_path: string | null
  model: string | null
  proxy_url: string | null
  launched_at: string | null
  status: 'success' | 'config_error' | 'launch_error' | null
}

type AppType = 'codex_cli' | 'codex_desktop' | 'claude_cli' | 'claude_desktop'

interface LaunchRequest {
  app_type: AppType
  model: string
}

interface SetPathRequest {
  install_path: string
}
```

## 错误处理

| 场景 | 处理方式 |
|---|---|
| 应用未安装（路径检测失败且无手动配置） | 返回错误，前端显示"未检测到该应用，请配置安装路径" |
| 配置文件写入失败 | 返回 `config_error` 状态，前端显示具体错误信息 |
| 进程启动失败 | 返回 `launch_error` 状态，前端显示启动失败提示 |
| 模型不在 provider_models 中 | 允许用户手动输入模型名（下拉框支持自定义输入） |

## 代理地址

代理地址从现有 settings 表中获取，默认为 `http://127.0.0.1:7860`。Codex 系列追加 `/v1` 后缀（`http://127.0.0.1:7860/v1`），Claude 系列不追加。

## 迁移文件

新增 `src-tauri/migrations/006_app_configs.sql`，包含建表语句。在 `src-tauri/src/db/init.rs` 中注册。
