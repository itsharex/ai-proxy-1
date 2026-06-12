# AI Proxy 服务化部署指南

本文档介绍如何将 AI Proxy 以 **无头服务（Headless Server）** 的方式部署运行，适用于团队共享、远程服务器、CI/CD 集成等场景。

## 概述

AI Proxy 支持两种运行形态：

| 功能 | 桌面版 | 服务版 |
|---|---|---|
| Tauri GUI | ✅ | ❌ |
| 系统托盘 | ✅ | ❌ |
| 开机自启 | ✅（macOS LaunchAgent） | 用 systemd / Docker |
| 自动更新检查 | ✅ | ❌ |
| 应用启动器 | ✅ | ❌ |
| MCP 导入/同步到本地应用 | ✅ | ❌ |
| 运行日志实时推送 | Tauri Emitter | WebSocket |
| 数据库路径 | Tauri API 自动管理 | 环境变量 / 命令行参数 |
| 管理 API 认证 | 不需要 | JWT 登录 |

服务版通过 Cargo `server` feature 编译，不依赖 Tauri，仅保留 Axum HTTP 服务、SQLite 数据库、JWT 认证和 Web 前端静态文件托管。

## 快速开始

### Docker Compose 部署（推荐）

前置条件：已安装 Docker 和 Docker Compose。

```bash
# 1. 复制环境变量模板
cp .env.example .env

# 2. 编辑 .env，至少设置以下两项
# MASTER_KEY=你的随机字符串（用于加密数据库中的 API Key）
# JWT_SECRET=你的随机字符串（用于 JWT 签名）
# ADMIN_PASSWORD=你的管理员密码（可选，未设置则首次启动随机生成）

# 3. 启动服务
docker-compose up -d

# 4. 查看日志（首次启动会打印生成的管理员密码）
docker-compose logs -f

# 5. 验证服务
# Web UI: http://localhost:7860
# 健康检查: curl http://localhost:7860/health
# 默认管理员: admin / (.env 中设置的密码)
```

`.env.example` 中的变量会在 `docker-compose.yml` 里自动映射为容器内部使用的 `AI_PROXY_*` 环境变量，一般不需要手动修改 `docker-compose.yml`。

### 停止与重启

```bash
# 停止
docker-compose down

# 停止并删除数据卷（谨慎操作）
docker-compose down -v

# 重启
docker-compose restart

# 查看状态
docker-compose ps
```

## docker run 示例

如果你不想使用 Docker Compose，也可以直接用 `docker run` 运行。

### 最小化运行

```bash
docker run -d \
  --name ai-proxy \
  -p 7860:7860 \
  -e AI_PROXY_MASTER_KEY="$(openssl rand -hex 32)" \
  -e AI_PROXY_JWT_SECRET="$(openssl rand -hex 32)" \
  mrhua382812/ai-proxy-server:latest
```

> 注意：`openssl rand -hex 32` 每次运行结果不同。首次启动后，后续重启必须保持 `MASTER_KEY` 和 `JWT_SECRET` 不变，否则已加密数据会无法解密、已有 Token 会失效。

### 带数据持久化

```bash
docker run -d \
  --name ai-proxy \
  -p 7860:7860 \
  -v ai-proxy-data:/data \
  -e AI_PROXY_MASTER_KEY="your-master-key" \
  -e AI_PROXY_JWT_SECRET="your-jwt-secret" \
  -e AI_PROXY_ADMIN_PASSWORD="your-admin-password" \
  mrhua382812/ai-proxy-server:latest
```

### 指定静态文件目录与端口

```bash
docker run -d \
  --name ai-proxy \
  -p 8080:7860 \
  -v ai-proxy-data:/data \
  -v /path/to/your/static:/app/static:ro \
  -e AI_PROXY_MASTER_KEY="your-master-key" \
  -e AI_PROXY_JWT_SECRET="your-jwt-secret" \
  -e AI_PROXY_STATIC_DIR=/app/static \
  mrhua382812/ai-proxy-server:latest
```

## 二进制部署

### 编译

```bash
cd src-tauri
cargo build --release --features server --no-default-features
```

编译产物位于 `src-tauri/target/release/ai-proxy-server`。

### 直接运行

```bash
export AI_PROXY_MASTER_KEY="your-master-key"
export AI_PROXY_JWT_SECRET="your-jwt-secret"
export AI_PROXY_ADMIN_PASSWORD="your-admin-password"

./target/release/ai-proxy-server \
  --host 0.0.0.0 \
  --port 7860 \
  --data-dir /var/lib/ai-proxy
```

### systemd 服务示例

创建 `/etc/systemd/system/ai-proxy.service`：

```ini
[Unit]
Description=AI Proxy Server
After=network.target

[Service]
Type=simple
User=ai-proxy
Group=ai-proxy
WorkingDirectory=/opt/ai-proxy
EnvironmentFile=/etc/ai-proxy/env
ExecStart=/opt/ai-proxy/ai-proxy-server --host 0.0.0.0 --port 7860 --data-dir /var/lib/ai-proxy
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

创建环境变量文件 `/etc/ai-proxy/env`：

```bash
AI_PROXY_MASTER_KEY=your-master-key
AI_PROXY_JWT_SECRET=your-jwt-secret
AI_PROXY_ADMIN_PASSWORD=your-admin-password
AI_PROXY_STATIC_DIR=/opt/ai-proxy/static
RUST_LOG=info
```

创建用户、目录并启动：

```bash
sudo useradd -r -s /usr/sbin/nologin ai-proxy
sudo mkdir -p /var/lib/ai-proxy /opt/ai-proxy
sudo chown -R ai-proxy:ai-proxy /var/lib/ai-proxy /opt/ai-proxy
sudo chmod 600 /etc/ai-proxy/env

sudo systemctl daemon-reload
sudo systemctl enable --now ai-proxy
sudo systemctl status ai-proxy
```

## 命令行参数

```
ai-proxy-server [OPTIONS]

Options:
  -h, --host <HOST>              监听地址 [default: 0.0.0.0]
  -p, --port <PORT>              监听端口 [default: 7860]
  -d, --data-dir <DATA_DIR>      数据目录 [default: /var/lib/ai-proxy]
      --admin-password <PASSWORD>  初始管理员密码
      --static-dir <DIR>         前端静态文件目录
```

参数也可以通过环境变量指定：`AI_PROXY_DATA_DIR`、`AI_PROXY_ADMIN_PASSWORD`、`AI_PROXY_STATIC_DIR`。

## 环境变量说明

### .env 文件（Docker Compose 使用）

`.env.example` 提供了 Docker Compose 部署时的变量模板：

| 变量名 | 说明 | 必需 |
|---|---|---|
| `MASTER_KEY` | 数据库加密主密钥，重启后必须保持一致 | 是 |
| `JWT_SECRET` | JWT 签名密钥，重启后必须保持一致 | 是 |
| `ADMIN_PASSWORD` | 初始管理员密码 | 否（未设置则首次启动随机生成） |
| `PORT` | 宿主机暴露端口（默认 7860） | 否 |
| `STATIC_DIR` | 前端静态文件目录（容器内，默认 `/app/static`） | 否 |
| `LOG_LEVEL` | 日志级别：trace/debug/info/warn/error（默认 info） | 否 |

### 容器 / 二进制实际读取的环境变量

服务进程实际读取的变量名带 `AI_PROXY_` 前缀。`docker-compose.yml` 已自动完成映射，手动运行 Docker 或二进制时需要直接设置：

| 变量名 | 说明 | 必需 |
|---|---|---|
| `AI_PROXY_MASTER_KEY` | 数据库加密主密钥 | 是 |
| `AI_PROXY_JWT_SECRET` | JWT 签名密钥 | 是 |
| `AI_PROXY_ADMIN_PASSWORD` | 初始管理员密码 | 否 |
| `AI_PROXY_DATA_DIR` | 数据目录（默认 `/var/lib/ai-proxy`） | 否 |
| `AI_PROXY_STATIC_DIR` | 前端静态文件目录 | 否 |
| `RUST_LOG` | 日志级别（默认 info） | 否 |

### 安全建议

- `MASTER_KEY` 和 `JWT_SECRET` 应使用足够长的随机字符串，例如：
  ```bash
  openssl rand -hex 32
  ```
- 一旦首次启动并写入加密数据后，**切勿更换 `MASTER_KEY`**，否则数据库中已加密的 API Key 将无法解密。
- `.env` 文件权限建议设置为 `600`。
- 生产环境建议通过反向代理提供 HTTPS，不要在公网直接暴露 HTTP 服务。

## API 认证

服务版有两层认证：

1. **管理 API 认证（JWT）**：保护 `/api/*`（除 `/api/auth/*`），用于 Web UI 和管理接口。
2. **代理 API 认证（可配置 API Key）**：保护 `/v1/*` 代理端点，用于客户端调用大模型 API。

### JWT 管理 API 认证

#### 登录获取 Token

```bash
curl -X POST http://localhost:7860/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-password"}'
```

响应：

```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 86400,
    "user": {
      "id": "...",
      "username": "admin",
      "role": "admin"
    }
  }
}
```

Token 有效期为 **24 小时**。

#### 使用 Token 访问管理 API

```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  http://localhost:7860/api/providers
```

#### 获取当前用户

```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  http://localhost:7860/api/auth/me
```

#### 登出

```bash
curl -X POST -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  http://localhost:7860/api/auth/logout
```

### 代理 API Key 认证

代理端点默认不强制认证。启用后，客户端需要在请求中携带 API Key。

#### 启用方式

1. 通过 Web UI 登录管理员账号。
2. 进入"设置"页面。
3. 设置以下两项：
   - `proxy_auth_enabled` = `true`
   - `proxy_auth_key` = 你期望的代理 API Key（建议随机字符串）
4. 保存设置后立即生效。

#### 客户端请求方式

支持两种 Header 形式：

```bash
# 方式一：Authorization: Bearer
curl -H "Authorization: Bearer your-proxy-api-key" \
  http://localhost:7860/v1/chat/completions

# 方式二：X-API-Key
curl -H "X-API-Key: your-proxy-api-key" \
  http://localhost:7860/v1/chat/completions
```

## 数据持久化

### 数据目录结构

服务版使用 SQLite 数据库存储配置、API Key、请求日志、用量统计等。默认数据目录：

- 二进制部署：`/var/lib/ai-proxy`
- Docker 部署：`/data`

数据库文件为 `{data_dir}/ai-proxy.db`，同时会生成 `-wal` 和 `-shm` 文件（WAL 模式）。

### Docker Volume

`docker-compose.yml` 中已定义命名卷：

```yaml
volumes:
  - ai-proxy-data:/data
```

数据会持久化在 Docker 管理的卷中。如需备份，可以：

```bash
# 复制数据库到宿主机
docker cp ai-proxy:/data/ai-proxy.db ./ai-proxy-backup.db
```

### 备份与恢复

```bash
# 备份（建议服务停止或数据库不忙时执行）
cp /var/lib/ai-proxy/ai-proxy.db /backup/ai-proxy-$(date +%Y%m%d).db

# 恢复
systemctl stop ai-proxy
cp /backup/ai-proxy-YYYYMMDD.db /var/lib/ai-proxy/ai-proxy.db
systemctl start ai-proxy
```

### 多实例共享数据库

多个服务实例可以挂载同一个数据目录或共享数据库文件，但需满足：

- 所有实例使用相同的 `AI_PROXY_MASTER_KEY`。
- 建议只用一个实例写入，其他实例只读，避免 SQLite 并发写入锁竞争。

## 客户端远程配置示例

将本地或远程部署的 AI Proxy 作为上游代理使用时，需要把客户端的 API Base URL 指向 `http(s)://<host>:7860`，并使用代理 API Key（如果启用了代理认证）。

### Codex CLI

编辑 `~/.codex/config.toml`：

```toml
[model_providers.ai-proxy]
name = "ai-proxy"
base_url = "http://your-server:7860"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "your-proxy-api-key"

model_provider = "ai-proxy"
```

如果启用代理认证，同时编辑 `~/.codex/auth.json`：

```json
{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "your-proxy-api-key"
}
```

### Claude CLI

编辑 `~/.claude/settings.json`：

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "http://your-server:7860",
    "ANTHROPIC_API_KEY": "your-proxy-api-key",
    "ANTHROPIC_MODEL": "claude-sonnet-4-6"
  }
}
```

### Claude Desktop

Claude Desktop 通过 Claude-3p 配置目录使用第三方网关。配置路径：

- macOS：`~/Library/Application Support/Claude-3p/`
- Windows：`%LOCALAPPDATA%\Claude-3p\`
- Linux：`~/.config/Claude-3p/`

创建/编辑 `claude_desktop_config.json`：

```json
{
  "deploymentMode": "3p"
}
```

创建 `configLibrary/<your-profile-id>.json`：

```json
{
  "disableDeploymentModeChooser": true,
  "inferenceGatewayApiKey": "your-proxy-api-key",
  "inferenceGatewayAuthScheme": "bearer",
  "inferenceGatewayBaseUrl": "http://your-server:7860",
  "inferenceProvider": "gateway",
  "inferenceModels": [
    { "name": "claude-haiku-4-5" },
    { "name": "claude-sonnet-4-6" },
    { "name": "claude-opus-4-7", "supports1m": true }
  ]
}
```

然后在 `configLibrary/_meta.json` 中注册该 profile。

> 注：AI Proxy 桌面版提供一键写入上述配置的功能；服务版需要手动编辑客户端配置文件。

### OpenCode CLI

编辑 `~/.config/opencode/opencode.json`：

```json
{
  "provider": {
    "AiProxy": {
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "apiKey": "your-proxy-api-key",
        "baseURL": "http://your-server:7860"
      },
      "models": {
        "gpt-4o": { "name": "gpt-4o" }
      }
    }
  }
}
```

### 通用 OpenAI 兼容客户端

```bash
export OPENAI_API_KEY="your-proxy-api-key"
export OPENAI_BASE_URL="http://your-server:7860/v1"
```

大多数支持自定义 `base_url` 的 OpenAI SDK / CLI 都可以按此方式配置。

## 反向代理 HTTPS 示例

### Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name ai-proxy.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://127.0.0.1:7860;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket 支持（运行日志实时推送）
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Caddy

```
ai-proxy.example.com {
    reverse_proxy localhost:7860
}
```

Caddy 会自动处理 HTTPS 证书和 WebSocket 升级。

### Traefik（Docker 标签方式）

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.ai-proxy.rule=Host(`ai-proxy.example.com`)"
  - "traefik.http.routers.ai-proxy.tls.certresolver=letsencrypt"
  - "traefik.http.services.ai-proxy.loadbalancer.server.port=7860"
```

### WebSocket 说明

运行日志实时推送端点为 `/api/runtime-logs/stream`。反向代理需要正确转发 `Upgrade` 和 `Connection` Header。

## 升级指南

### Docker 升级

```bash
# 拉取最新镜像
docker-compose pull

# 重新创建容器（数据卷保留）
docker-compose up -d
```

### 二进制升级

```bash
# 1. 备份数据目录
cp -r /var/lib/ai-proxy /var/lib/ai-proxy-backup

# 2. 停止服务
sudo systemctl stop ai-proxy

# 3. 替换二进制
sudo cp ./target/release/ai-proxy-server /opt/ai-proxy/ai-proxy-server

# 4. 启动服务
sudo systemctl start ai-proxy
```

### 数据库迁移

数据库迁移在启动时自动执行，无需手动干预。迁移文件位于 `src-tauri/migrations/`，当前编号为 001–016。升级后首次启动会自动应用新增的迁移。

### 回滚

如果升级后出现问题，可以停止服务并恢复数据目录备份：

```bash
sudo systemctl stop ai-proxy
sudo rm -rf /var/lib/ai-proxy
sudo cp -r /var/lib/ai-proxy-backup /var/lib/ai-proxy
sudo systemctl start ai-proxy
```

> 注意：回滚到旧版本二进制时，如果数据库已经被新版本迁移修改，旧版本可能无法正确读取。建议升级前务必备份数据目录。

## 故障排查

### 服务无法启动

**现象**：容器反复重启，日志显示 `FATAL: AI_PROXY_JWT_SECRET environment variable is not set.`

**解决**：确保 `.env` 中设置了 `JWT_SECRET`，或运行命令中设置了 `AI_PROXY_JWT_SECRET`。

### 端口被占用

**现象**：启动时报 `Address already in use`

**解决**：

```bash
# 查找占用 7860 端口的进程
sudo lsof -i :7860
# 或修改 docker-compose.yml / 命令行参数使用其他端口
```

### 管理 API 返回 401

**可能原因**：

- 请求头未携带 `Authorization: Bearer <token>`。
- Token 已过期（24 小时），需要重新登录获取。
- `AI_PROXY_JWT_SECRET` 与生成 Token 时不一致（例如重建容器后更换了 secret）。

### 代理 API 返回 401

**可能原因**：

- 已在设置中启用 `proxy_auth_enabled`，但请求未携带 API Key。
- 请求中使用的 Key 与 `proxy_auth_key` 不一致。

### 数据库锁定

SQLite 使用 WAL 模式，通常不会出现长时间锁定。如果遇到 `database is locked`，检查是否有其他进程打开了数据库文件，或是否有多个实例同时写入同一数据库。

### 查看详细日志

```bash
# Docker
docker-compose logs -f

# 设置更详细的日志级别
docker-compose down
# 编辑 .env，设置 LOG_LEVEL=debug
docker-compose up -d

# systemd
sudo journalctl -u ai-proxy -f
```

## 参考

### 环境变量速查表

| 来源变量（.env） | 进程变量 | 必需 | 默认值 |
|---|---|---|---|
| `MASTER_KEY` | `AI_PROXY_MASTER_KEY` | 是 | - |
| `JWT_SECRET` | `AI_PROXY_JWT_SECRET` | 是 | - |
| `ADMIN_PASSWORD` | `AI_PROXY_ADMIN_PASSWORD` | 否 | 随机 UUID |
| - | `AI_PROXY_DATA_DIR` | 否 | `/var/lib/ai-proxy` |
| `STATIC_DIR` | `AI_PROXY_STATIC_DIR` | 否 | `/app/static`（Docker） |
| `LOG_LEVEL` | `RUST_LOG` | 否 | `info` |
| `PORT` | - | 否 | `7860`（仅控制 docker-compose 端口映射） |

### 端点速查表

| 端点 | 说明 | 认证 |
|---|---|---|
| `GET /health` | 健康检查 | 无 |
| `POST /api/auth/login` | 管理员登录 | 无 |
| `POST /api/auth/logout` | 登出 | JWT |
| `GET /api/auth/me` | 当前用户 | JWT |
| `/api/*` | 管理 API | JWT（除 `/api/auth/*`） |
| `/v1/*` | 代理端点 | 可选 API Key |

### 相关文件

- `Dockerfile` — 多阶段构建定义
- `docker-compose.yml` — Docker Compose 编排
- `.env.example` — 环境变量模板
- `.github/workflows/build-docker.yml` — Docker 镜像 CI
- `src-tauri/src/server_main.rs` — 服务版入口
- `src-tauri/src/auth/` — JWT 认证实现
- `src-tauri/src/server/middleware.rs` — 代理 API Key 认证
- `src-tauri/src/db/init.rs` — 数据库初始化与迁移
