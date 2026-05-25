# Simplify Routing: Provider Format + Model List Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the complex model_routes + endpoints routing with a simple Provider-owned model list, where format is a property of the provider and endpoint paths are auto-derived.

**Architecture:** Each provider has one `format` field (completions/responses/anthropic/gemini) and a list of models. Routing looks up a model by name in provider_models, gets the provider's format, auto-derives the endpoint path, and uses IR conversion to translate between client and provider formats.

**Tech Stack:** Rust/axum (backend), Vue 3 + Naive UI (frontend), SQLite/sqlx (database)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src-tauri/migrations/003_simplify_routing.sql` | DB schema: add format to providers, create provider_models, drop old tables |
| `src-tauri/src/provider/endpoint.rs` | Remove ProviderEndpoint, add ProviderModel struct |
| `src-tauri/src/provider/manager.rs` | Rewrite find_for_model, update list/get_by_id, remove endpoint queries |
| `src-tauri/src/server/api.rs` | Rewrite provider CRUD with models, remove routes endpoints |
| `src-tauri/src/server/router.rs` | Remove /api/routes |
| `src/types/index.ts` | Update TypeScript interfaces |
| `src/views/Providers.vue` | Add format selector + model list editor |
| `src/views/Models.vue` | Rewrite as read-only model overview |

---

### Task 1: Database Migration

**Files:**
- Create: `src-tauri/migrations/003_simplify_routing.sql`
- Modify: `src-tauri/src/db/init.rs`

- [ ] **Step 1: Create migration file**

```sql
-- Add format column to providers
ALTER TABLE providers ADD COLUMN format TEXT NOT NULL DEFAULT 'completions'
  CHECK(format IN ('completions', 'responses', 'anthropic', 'gemini'));

-- Create provider_models table
CREATE TABLE IF NOT EXISTS provider_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  model_name TEXT NOT NULL,
  target_model TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(provider_id, model_name)
);

-- Migrate data from model_routes to provider_models
INSERT OR IGNORE INTO provider_models (id, provider_id, model_name, target_model, enabled, created_at)
SELECT
  id,
  provider_id,
  model_pattern,
  target_model,
  1,
  COALESCE(created_at, datetime('now'))
FROM model_routes;

-- Migrate target_format from model_routes to providers
-- Only do this for providers that have exactly one route targeting them
UPDATE providers SET format = (
  SELECT mr.target_format FROM model_routes mr
  WHERE mr.provider_id = providers.id
  LIMIT 1
) WHERE format = 'completions' AND EXISTS (
  SELECT 1 FROM model_routes WHERE model_routes.provider_id = providers.id
);

-- Drop old tables
DROP TABLE IF EXISTS endpoints;
DROP TABLE IF EXISTS model_routes;
```

- [ ] **Step 2: Register migration in init.rs**

In `src-tauri/src/db/init.rs`, add after migration2:

```rust
let migration3 = include_str!("../../migrations/003_simplify_routing.sql");
sqlx::query(migration3).execute(pool).await?;
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/003_simplify_routing.sql src-tauri/src/db/init.rs
git commit -m "feat: add migration for provider format and provider_models table"
```

---

### Task 2: Update Rust Data Structures

**Files:**
- Modify: `src-tauri/src/provider/endpoint.rs`
- Modify: `src-tauri/src/provider/mod.rs`

- [ ] **Step 1: Rewrite endpoint.rs with new structs**

Replace entire `src-tauri/src/provider/endpoint.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub format: String,
    pub auth_type: String,
    pub auth_header: String,
    pub models: Vec<ProviderModel>,
    pub api_keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    pub provider_id: String,
    pub model_name: String,
    pub target_model: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub label: String,
    pub is_active: bool,
    pub usage_count: u32,
    pub last_used_at: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 2: Update mod.rs**

Keep as-is — it already re-exports the submodule:

```rust
pub mod manager;
pub mod endpoint;
```

The file `endpoint.rs` still exists, just with different content. The module name stays the same for minimal disruption.

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: FAIL — `manager.rs` references `ProviderEndpoint`, `DbEndpoint`, and endpoint-related queries. This is expected; Task 3 fixes it.

---

### Task 3: Rewrite Routing Logic (manager.rs)

**Files:**
- Modify: `src-tauri/src/provider/manager.rs`

- [ ] **Step 1: Rewrite manager.rs**

Replace entire file. Key changes:
- Remove `DbEndpoint`, `ProviderEndpoint` references
- Add `DbProviderModel` row struct
- Add `format` to `DbProvider`
- Rewrite `find_for_model` to query `provider_models` + `providers`
- Remove endpoint table queries from `list()` and `get_by_id()`
- Keep `default_path_for_format` unchanged

```rust
use crate::converter::ir::ClientFormat;
use crate::db::get_pool;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::endpoint::{ApiKeyInfo, Provider, ProviderModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedRoute {
    pub provider_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: String,
    pub target_format: ClientFormat,
    pub target_model: String,
    pub endpoint_path: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DbProvider {
    id: String,
    name: String,
    base_url: String,
    format: String,
    auth_type: String,
    auth_header: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DbProviderModel {
    id: String,
    provider_id: String,
    model_name: String,
    target_model: Option<String>,
    enabled: i64,
    created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DbApiKeyInfo {
    id: String,
    label: String,
    is_active: i64,
    usage_count: i64,
    last_used_at: Option<String>,
    created_at: String,
}

pub struct ProviderManager;

impl ProviderManager {
    pub async fn list() -> Result<Vec<Provider>, crate::error::ProxyError> {
        let pool = get_pool().await;

        let db_providers: Vec<DbProvider> =
            sqlx::query_as("SELECT id, name, base_url, format, auth_type, auth_header FROM providers ORDER BY name")
                .fetch_all(pool)
                .await
                .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        let mut providers = Vec::new();
        for p in db_providers {
            let models = Self::fetch_models(&p.id).await?;
            let api_keys = Self::fetch_api_keys_info(&p.id).await?;
            providers.push(Provider {
                id: p.id,
                name: p.name,
                base_url: p.base_url,
                format: p.format,
                auth_type: p.auth_type,
                auth_header: p.auth_header,
                models,
                api_keys,
            });
        }

        Ok(providers)
    }

    pub async fn get_by_id(provider_id: &str) -> Result<Provider, crate::error::ProxyError> {
        let pool = get_pool().await;

        let p: DbProvider = sqlx::query_as(
            "SELECT id, name, base_url, format, auth_type, auth_header FROM providers WHERE id = ?",
        )
        .bind(provider_id)
        .fetch_one(pool)
        .await
        .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        let models = Self::fetch_models(&p.id).await?;
        let api_keys = Self::fetch_api_keys_info(&p.id).await?;

        Ok(Provider {
            id: p.id,
            name: p.name,
            base_url: p.base_url,
            format: p.format,
            auth_type: p.auth_type,
            auth_header: p.auth_header,
            models,
            api_keys,
        })
    }

    pub async fn find_for_model(model: &str) -> Result<ResolvedRoute, crate::error::ProxyError> {
        let pool = get_pool().await;

        info!("Looking up route for model: {}", model);

        let db_models: Vec<DbProviderModel> = sqlx::query_as(
            "SELECT id, provider_id, model_name, target_model, enabled, created_at
             FROM provider_models WHERE enabled = 1 ORDER BY created_at",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        let matched = db_models
            .iter()
            .find(|m| m.model_name == model)
            .ok_or_else(|| {
                crate::error::ProxyError::Routing(format!("no provider found for model '{}'", model))
            })?;

        let pool = get_pool().await;
        let provider: DbProvider = sqlx::query_as(
            "SELECT id, name, base_url, format, auth_type, auth_header FROM providers WHERE id = ?",
        )
        .bind(&matched.provider_id)
        .fetch_one(pool)
        .await
        .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        let target_model = matched
            .target_model
            .clone()
            .unwrap_or_else(|| matched.model_name.clone());

        let target_format = parse_client_format(&provider.format)
            .map_err(|e| crate::error::ProxyError::Parse(e))?;

        let endpoint_path = default_path_for_format(&target_format, &target_model);

        info!(
            "Route resolved: {} -> {} ({}) via {}",
            model, target_model, provider.format, provider.name
        );

        Ok(ResolvedRoute {
            provider_id: provider.id,
            provider_name: provider.name,
            base_url: provider.base_url,
            auth_type: provider.auth_type,
            auth_header: provider.auth_header,
            target_format,
            target_model,
            endpoint_path,
        })
    }

    async fn fetch_models(provider_id: &str) -> Result<Vec<ProviderModel>, crate::error::ProxyError> {
        let pool = get_pool().await;
        let rows: Vec<DbProviderModel> = sqlx::query_as(
            "SELECT id, provider_id, model_name, target_model, enabled, created_at
             FROM provider_models WHERE provider_id = ? ORDER BY model_name",
        )
        .bind(provider_id)
        .fetch_all(pool)
        .await
        .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ProviderModel {
                id: r.id,
                provider_id: r.provider_id,
                model_name: r.model_name,
                target_model: r.target_model,
                enabled: r.enabled != 0,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn fetch_api_keys_info(provider_id: &str) -> Result<Vec<ApiKeyInfo>, crate::error::ProxyError> {
        let pool = get_pool().await;
        let rows: Vec<DbApiKeyInfo> = sqlx::query_as(
            "SELECT id, label, is_active, usage_count, last_used_at, created_at
             FROM api_keys WHERE provider_id = ? ORDER BY created_at",
        )
        .bind(provider_id)
        .fetch_all(pool)
        .await
        .map_err(|e| crate::error::ProxyError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ApiKeyInfo {
                id: r.id,
                label: r.label,
                is_active: r.is_active != 0,
                usage_count: r.usage_count as u32,
                last_used_at: r.last_used_at,
                created_at: r.created_at,
            })
            .collect())
    }
}

fn parse_client_format(format: &str) -> Result<ClientFormat, String> {
    match format {
        "completions" => Ok(ClientFormat::Completions),
        "responses" => Ok(ClientFormat::Responses),
        "anthropic" => Ok(ClientFormat::Anthropic),
        "gemini" => Ok(ClientFormat::Gemini),
        other => Err(format!("unknown format: {}", other)),
    }
}

fn default_path_for_format(format: &ClientFormat, target_model: &str) -> String {
    match format {
        ClientFormat::Completions => "/v1/chat/completions".to_string(),
        ClientFormat::Responses => "/v1/responses".to_string(),
        ClientFormat::Anthropic => "/v1/messages".to_string(),
        ClientFormat::Gemini => format!("/v1beta/models/{}:generateContent", target_model),
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: FAIL — `api.rs` still references `ProviderEndpoint` and routes endpoints. This is expected; Task 4 fixes it.

---

### Task 4: Rewrite API Handlers

**Files:**
- Modify: `src-tauri/src/server/api.rs`
- Modify: `src-tauri/src/server/router.rs`

- [ ] **Step 1: Rewrite api.rs provider CRUD and remove routes endpoints**

In `src-tauri/src/server/api.rs`, the changes are:

1. **Update `create_provider`** to accept `format` field and `models` array, create provider_models rows, no endpoints
2. **Update `update_provider`** to accept `format` field and `models` array, sync models (delete+reinsert)
3. **Update `list_providers`** response to include `format` and `models` (already handled by ProviderManager::list)
4. **Remove** `list_routes`, `create_route`, `update_route`, `test_route`, `delete_route` functions
5. **Update `api_routes()`** to remove all `/api/routes*` endpoints
6. **Remove** all `ProviderEndpoint` references

The updated `create_provider` handler:

```rust
#[derive(Deserialize)]
struct CreateProviderBody {
    name: String,
    base_url: String,
    format: String,
    auth_type: Option<String>,
    auth_header: Option<String>,
    api_key: String,
    models: Vec<CreateModelBody>,
}

#[derive(Deserialize)]
struct CreateModelBody {
    model_name: String,
    target_model: Option<String>,
}
```

Create logic:
```rust
async fn create_provider(Json(body): Json<CreateProviderBody>) -> Response {
    // ... validate ...
    let pool = get_pool().await;
    let id = uuid::Uuid::new_v4().to_string();
    let auth_type = body.auth_type.unwrap_or_else(|| "bearer".into());
    let auth_header = body.auth_header.unwrap_or_else(|| "Authorization".into());

    // Insert provider with format
    sqlx::query("INSERT INTO providers (id, name, base_url, format, auth_type, auth_header) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(&body.name).bind(&body.base_url).bind(&body.format)
        .bind(&auth_type).bind(&auth_header)
        .execute(pool).await;

    // Insert models
    for m in &body.models {
        let model_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO provider_models (id, provider_id, model_name, target_model) VALUES (?, ?, ?, ?)")
            .bind(&model_id).bind(&id).bind(&m.model_name).bind(&m.target_model)
            .execute(pool).await;
    }

    // Encrypt and store API key (same as current logic)
    // ...

    // Return provider with models
    let provider = ProviderManager::get_by_id(&id).await;
    ok(provider)
}
```

The updated `update_provider` handler:

```rust
#[derive(Deserialize)]
struct UpdateProviderBody {
    name: Option<String>,
    base_url: Option<String>,
    format: Option<String>,
    api_key: Option<String>,
    models: Option<Vec<CreateModelBody>>,
}
```

Update logic for models: when `models` is provided, delete all existing models for the provider and re-insert the new set.

**Remove these functions entirely:** `list_routes`, `create_route`, `update_route`, `test_route`, `delete_route`

**Update `api_routes()`** to:
```rust
pub fn api_routes() -> axum::Router {
    axum::Router::new()
        .route("/providers", axum::routing::get(list_providers).post(create_provider))
        .route("/providers/:id", routing::put(update_provider).delete(delete_provider))
        // /api/routes endpoints REMOVED
        .route("/logs", axum::routing::get(list_logs))
        .route("/logs/:id", axum::routing::get(get_log))
        .route("/usage", axum::routing::get(get_usage))
        .route("/rules", axum::routing::get(list_rules).post(create_rule))
        .route("/rules/:id", routing::put(update_rule).delete(delete_rule))
        .route("/settings", axum::routing::get(get_settings).put(update_settings))
}
```

- [ ] **Step 2: Update router.rs**

No changes needed to `router.rs` — the `/api/routes` endpoints are managed in `api.rs`'s `api_routes()`, which we just updated. The proxy routes (`/v1/*`) stay unchanged.

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/server/api.rs src-tauri/src/provider/endpoint.rs src-tauri/src/provider/manager.rs src-tauri/src/provider/mod.rs
git commit -m "feat: rewrite provider CRUD with format + models, remove routes API"
```

---

### Task 5: Update Frontend Types

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: Update TypeScript interfaces**

Replace `Provider`, remove `ProviderEndpoint`, replace `ModelRoute`:

```typescript
export interface Provider {
  id: string
  name: string
  base_url: string
  format: 'completions' | 'responses' | 'anthropic' | 'gemini'
  auth_type: string
  auth_header: string
  models: ProviderModel[]
  api_keys: ApiKeyInfo[]
}

export interface ProviderModel {
  id: string
  provider_id: string
  model_name: string
  target_model: string | null
  enabled: boolean
  created_at: string
}

// Keep ApiKeyInfo, RequestLog, UsageSummary, InterceptorRule, RuleCondition, RuleAction unchanged
// REMOVE ProviderEndpoint and ModelRoute interfaces
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && npx vue-tsc --noEmit 2>&1 | head -20`
Expected: FAIL — Providers.vue and Models.vue reference old types. Task 6 fixes this.

---

### Task 6: Rewrite Providers.vue with Format + Model Management

**Files:**
- Modify: `src/views/Providers.vue`

- [ ] **Step 1: Rewrite Providers.vue**

The new Providers.vue must:
1. List providers in a table with columns: name, base_url, format (tag), models count, actions
2. Create/edit modal with: name, base_url, format (select), api_key, auth_type, auth_header
3. **Model list editor** within the provider form — a dynamic list of model_name + target_model pairs, with add/remove buttons
4. When creating, POST models array along with provider data
5. When editing, PUT models array to sync

Key UI structure:

```vue
<template>
  <n-space vertical size="large">
    <n-card title="供应商管理">
      <template #header-extra>
        <n-button type="primary" @click="openCreate">添加供应商</n-button>
      </template>
      <n-data-table :columns="columns" :data="providers" />
    </n-card>

    <n-modal v-model:show="showModal" :title="editingId ? '编辑供应商' : '添加供应商'" preset="card" style="width: 640px">
      <n-form label-placement="left" label-width="100">
        <n-form-item label="名称">
          <n-input v-model:value="form.name" placeholder="OpenAI" />
        </n-form-item>
        <n-form-item label="Base URL">
          <n-input v-model:value="form.base_url" placeholder="https://api.openai.com" />
        </n-form-item>
        <n-form-item label="API 格式">
          <n-select v-model:value="form.format" :options="formatOptions" />
        </n-form-item>
        <n-form-item label="API Key">
          <n-input v-model:value="form.api_key" type="password" show-password-on="click"
            :placeholder="editingId ? '留空则不修改' : '输入 API Key'" />
        </n-form-item>
        <n-form-item label="认证类型">
          <n-select v-model:value="form.auth_type" :options="authTypeOptions" />
        </n-form-item>
        <n-form-item label="认证头" v-if="form.auth_type === 'custom'">
          <n-input v-model:value="form.auth_header" placeholder="X-API-Key" />
        </n-form-item>

        <n-divider>模型列表</n-divider>
        <div v-for="(m, i) in form.models" :key="i" style="display: flex; gap: 8px; margin-bottom: 8px">
          <n-input v-model:value="m.model_name" placeholder="模型名称（如 gpt-4o）" style="flex: 1" />
          <n-input v-model:value="m.target_model" placeholder="上游模型名（可选）" style="flex: 1" />
          <n-button @click="form.models.splice(i, 1)" quaternary type="error">删除</n-button>
        </div>
        <n-button @click="form.models.push({ model_name: '', target_model: '' })" dashed block>添加模型</n-button>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showModal = false">取消</n-button>
          <n-button type="primary" @click="handleSave" :loading="saving">保存</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>
```

Script data:
- `formatOptions`: `[{ label: 'OpenAI Completions', value: 'completions' }, { label: 'OpenAI Responses', value: 'responses' }, { label: 'Anthropic', value: 'anthropic' }, { label: 'Google Gemini', value: 'gemini' }]`
- `authTypeOptions`: `[{ label: 'Bearer Token', value: 'bearer' }, { label: 'Custom Header', value: 'custom' }]`
- `form` reactive object: `{ name, base_url, format, api_key, auth_type, auth_header, models: [] }`
- `openCreate()`: reset form, show modal
- `openEdit(provider)`: populate form from provider data, show modal
- `handleSave()`: validate at least name/base_url/api_key (for create), POST or PUT

- [ ] **Step 2: Verify the page renders**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && npx vue-tsc --noEmit`
Expected: May still fail due to Models.vue. Fix in Task 7.

- [ ] **Step 3: Commit**

```bash
git add src/views/Providers.vue src/types/index.ts
git commit -m "feat: rewrite Providers.vue with format selector and model list editor"
```

---

### Task 7: Rewrite Models.vue as Read-Only Overview

**Files:**
- Modify: `src/views/Models.vue`

- [ ] **Step 1: Rewrite Models.vue**

Convert to a read-only model overview page. Fetch all providers, flatten their models, display in a single table.

```vue
<template>
  <n-space vertical size="large">
    <n-card title="模型总览">
      <template #header-extra>
        <n-text depth="3">模型在供应商中配置</n-text>
      </template>
      <n-data-table :columns="columns" :data="allModels" />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '../api'
import type { Provider, ProviderModel } from '../types'

const router = useRouter()
const providers = ref<Provider[]>([])

const allModels = computed(() => {
  const models: Array<ProviderModel & { provider_name: string; provider_format: string }> = []
  for (const p of providers.value) {
    for (const m of p.models) {
      models.push({ ...m, provider_name: p.name, provider_format: p.format })
    }
  }
  return models
})

const columns = [
  { title: '模型名称', key: 'model_name' },
  { title: '上游模型', key: 'target_model' },
  { title: '供应商', key: 'provider_name' },
  { title: '格式', key: 'provider_format' },
  { title: '状态', key: 'enabled', render: (row: any) => row.enabled ? '启用' : '禁用' },
]

async function loadData() {
  try {
    providers.value = await api<Provider[]>('/api/providers')
  } catch (e) {
    console.error('Failed to load providers:', e)
  }
}

onMounted(loadData)
</script>
```

- [ ] **Step 2: Verify full build**

Run: `cd src-tauri && cargo build && cd .. && npx vue-tsc --noEmit`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/views/Models.vue
git commit -m "refactor: convert Models.vue to read-only model overview"
```

---

### Task 8: Update App Navigation

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Update sidebar label**

In `src/App.vue`, change the "模型路由" menu item label to "模型总览" to reflect its new read-only nature:

Find: `{ label: '模型路由', key: '/models', icon: renderIcon(GitBranchOutline) }`
Replace: `{ label: '模型总览', key: '/models', icon: renderIcon(GitBranchOutline) }`

- [ ] **Step 2: Remove unused store import if any**

Check `src/stores/providers.ts` — it should still work as-is since it just calls `/api/providers`.

- [ ] **Step 3: Verify and commit**

```bash
git add src/App.vue
git commit -m "chore: update navigation label from 模型路由 to 模型总览"
```

---

### Task 9: Integration Testing

**Files:**
- Verify: All modified files

- [ ] **Step 1: Full build**

Run: `cd src-tauri && cargo build && cargo test`
Expected: All tests pass

- [ ] **Step 2: Start app and test provider creation**

Run: `cd src-tauri && cargo tauri dev`

Test via browser:
1. Open the app, go to Suppliers page
2. Create a new provider with format "anthropic", base_url "https://api.anthropic.com", and add model "claude-3-5-sonnet"
3. Verify the provider appears in the list with format tag and model count
4. Edit the provider and verify models are loaded correctly

- [ ] **Step 3: Test proxy routing**

Test via curl:
```bash
# Should route to the anthropic provider
curl -X POST http://127.0.0.1:7860/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-3-5-sonnet","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

Expected: Request is routed to the Anthropic provider, format conversion happens (Completions -> Anthropic), and response is converted back.

- [ ] **Step 4: Test Models overview page**

1. Go to Models page
2. Verify it shows all models from all providers in a flat table
3. Verify provider name and format columns are populated

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "test: verify simplified routing with provider format + model list"
```
