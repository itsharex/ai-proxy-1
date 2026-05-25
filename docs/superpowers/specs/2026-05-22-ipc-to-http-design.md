# IPC → HTTP 管理接口设计

## 背景

当前前端通过 Tauri IPC (`invoke()`) 调用后端 17 个命令。存在以下问题：

1. **调试困难**：IPC 调用无法在浏览器 DevTools 中观察，类型不匹配时前端静默卡死
2. **架构耦合**：前端强依赖 Tauri runtime，无法脱离 Tauri 独立运行
3. **已有 HTTP 服务**：axum 服务器已在 `127.0.0.1:7860` 运行代理转发，完全可以复用

## 设计

### 核心思路

在现有 axum 服务器上新增 `/api/*` 管理路由，前端用标准 `fetch()` 调用。
仅保留 1 个 IPC 命令 `get_api_config` 用于获取服务端地址。

### IPC 最小化

保留唯一 IPC 命令：

```rust
#[tauri::command]
fn get_api_config() -> String {
    // 从数据库读取 host:port，返回 "http://127.0.0.1:7860"
}
```

调用时机：
- 应用启动时获取一次
- 修改设置后获取一次（刷新 baseUrl）

### API 端点

所有端点走 `/api/*`，RESTful 风格，无认证（本地绑定）。

#### 供应商管理

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/providers` | GET | 获取供应商列表 |
| `/api/providers` | POST | 创建供应商 |
| `/api/providers/:id` | PUT | 更新供应商 |
| `/api/providers/:id` | DELETE | 删除供应商 |

#### API Key 管理

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/providers/:id/keys` | GET | 获取供应商的 API Key 列表 |
| `/api/providers/:id/keys` | POST | 添加 API Key |
| `/api/keys/:id` | DELETE | 删除 API Key |

#### 模型路由

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/routes` | GET | 获取路由列表 |
| `/api/routes` | POST | 创建路由 |
| `/api/routes/:id` | DELETE | 删除路由 |

#### 请求日志

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/logs?page=1&limit=20` | GET | 获取日志列表（分页） |
| `/api/logs/:id` | GET | 获取日志详情 |

#### 用量统计

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/usage?days=7` | GET | 获取用量统计 |

#### 拦截规则

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/rules` | GET | 获取规则列表 |
| `/api/rules` | POST | 创建规则 |
| `/api/rules/:id` | PUT | 更新规则 |
| `/api/rules/:id` | DELETE | 删除规则 |

#### 设置

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/settings` | GET | 获取设置 |
| `/api/settings` | PUT | 更新设置 |

### 统一响应格式

```typescript
interface ApiResponse<T> {
  success: boolean
  data: T
  error?: string
}
```

成功：`{ "success": true, "data": [...] }`
失败：`{ "success": false, "error": "描述信息" }`

### Rust 端变更

1. 新增 `src-tauri/src/server/api.rs` — 管理 API 路由注册
2. 将 6 个 IPC cmd 文件转为 axum handler（`Path<T>`, `Query<T>`, `Json<T>` extract）
3. `server/router.rs` 合并管理路由：`Router::new().nest("/api", api_routes)`
4. `lib.rs` 的 `invoke_handler` 仅注册 `get_api_config` 一个命令
5. 删除 `ipc/` 模块

### 前端变更

1. 新增 `src/api/index.ts` — API 客户端

```typescript
let baseUrl = ''

export async function initApi() {
  baseUrl = await invoke<string>('get_api_config')
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${baseUrl}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  const body = await res.json()
  if (!body.success) throw new Error(body.error || `API error: ${res.status}`)
  return body.data as T
}
```

2. 各页面将 `invoke('xxx')` 替换为 `api('/api/xxx')`
3. App.vue `onMounted` 中调用 `initApi()`
4. Settings 页面保存后调用 `initApi()` 刷新 baseUrl

### 开发模式

Vite 代理配置（仅开发时）：

```typescript
// vite.config.ts
server: {
  proxy: {
    '/api': 'http://127.0.0.1:7860'
  }
}
```

生产模式下 Tauri WebView 直接 fetch 完整 URL（自定义协议无跨域限制）。

## 影响范围

| 文件 | 变更 |
|------|------|
| `src-tauri/src/ipc/*.rs` | 删除，转为 HTTP handler |
| `src-tauri/src/server/api.rs` | 新增，管理 API 路由 |
| `src-tauri/src/server/router.rs` | 合并管理路由 |
| `src-tauri/src/lib.rs` | invoke_handler 仅注册 1 个命令 |
| `src/api/index.ts` | 新增，API 客户端 |
| `src/views/*.vue` | invoke → api() 替换 |
| `src/stores/providers.ts` | invoke → api() 替换 |
| `vite.config.ts` | 添加开发代理 |
