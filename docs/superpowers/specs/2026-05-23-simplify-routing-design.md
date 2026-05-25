# 简化路由设计：Provider 绑定 Format + 模型列表

## 问题

当前路由配置过于复杂：
- `model_routes` 表需要手动指定 `target_format`、`endpoint_path`
- `endpoints` 表存储 per-provider 的格式和路径
- 用户需要理解 format 概念才能完成配置
- 同一个 provider 的多个配置分散在多张表中

实际上，一个 provider 的 API 格式是固定的（OpenAI 用 completions，Anthropic 用 anthropic），endpoint path 可以从格式自动推导，无需用户手动配置。

## 设计

### 数据模型变更

**providers 表扩展**（新增 `format` 列）：
```sql
ALTER TABLE providers ADD COLUMN format TEXT NOT NULL DEFAULT 'completions'
  CHECK(format IN ('completions', 'responses', 'anthropic', 'gemini'));
```

**新建 provider_models 表**（替代 model_routes + endpoints）：
```sql
CREATE TABLE provider_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  model_name TEXT NOT NULL,       -- 客户端使用的模型名，如 "claude-3-5-sonnet"
  target_model TEXT,              -- 发给上游的模型名，为空则使用 model_name
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(provider_id, model_name)
);
```

**删除表**：
- `model_routes` — 被 `provider_models` 替代
- `endpoints` — 由 provider.format 自动推导

### 路由逻辑

**自动推导 endpoint path 的函数**（已存在于 `manager.rs` 的 `default_path_for_format`）：
```
completions -> /v1/chat/completions
responses   -> /v1/responses
anthropic   -> /v1/messages
gemini      -> /v1beta/models/{target_model}:generateContent
```

**路由查找流程**：
1. 客户端请求到达，从 URL 路径检测 client_format
2. 从请求 body 解析出 model_name
3. 查询 `provider_models WHERE model_name = ? AND enabled = 1`
4. 如果多条记录（多个 provider 有同名模型），使用 KeyRotation 已有的策略选择
5. 从 provider 取 `format`、`base_url`、`auth_*`
6. 自动推导 `endpoint_path` = `default_path_for_format(format, target_model)`
7. IR 转换 client_format -> provider.format，转发请求

### API 变更

**Provider CRUD**（`/api/providers`）：
- 创建/更新 provider 时包含 `format` 字段和 `models` 数组
- 响应中包含关联的模型列表

```
POST /api/providers
{
  "name": "Anthropic",
  "base_url": "https://api.anthropic.com",
  "format": "anthropic",
  "auth_type": "x-api-key",
  "auth_header": "x-api-key",
  "api_key": "sk-ant-...",
  "models": [
    { "model_name": "claude-3-5-sonnet", "target_model": "claude-3-5-sonnet-20241022" },
    { "model_name": "claude-3-opus" }
  ]
}
```

**模型 CRUD**（`/api/providers/:id/models`）：
- 新增端点管理 provider 下的模型
- GET/POST/PUT/DELETE 操作

**删除的路由端点**：
- `/api/routes` — 不再需要独立路由管理

### 前端变更

**Providers 页面**：
- provider 表单新增 `format` 下拉选择
- provider 详情/编辑中包含模型列表管理（增删改）
- 去掉独立的 Models 页面，或改为只读的模型总览视图

**Models 页面**（可选保留）：
- 改为全局模型总览：展示所有 provider 的模型列表
- 显示每个模型属于哪个 provider、provider 的 format
- 点击可跳转到 provider 编辑

### 文件变更清单

| 文件 | 变更 |
|------|------|
| `migrations/003_simplify_routing.sql` | 新建 provider_models 表，providers 加 format，删除 model_routes 和 endpoints |
| `src-tauri/src/provider/manager.rs` | 重写 `find_for_model`，查 provider_models + providers |
| `src-tauri/src/provider/mod.rs` | 更新 Provider struct 加 format 字段 |
| `src-tauri/src/server/api.rs` | 重写 provider CRUD 含模型管理，删除 routes 端点 |
| `src-tauri/src/server/router.rs` | 去掉 `/api/routes` 路由 |
| `src/views/Providers.vue` | 加 format 选择和模型列表管理 |
| `src/views/Models.vue` | 改为只读模型总览或合并到 Providers |
| `src/types/index.ts` | 更新类型定义 |

### 迁移策略

- 新建 migration 003
- 从 model_routes 迁移数据到 provider_models
- 保留旧表直到确认无误后删除
