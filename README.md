> 本项目进入维护模式：后续主要跟随上游 Vaultwarden 的协议、客户端兼容性和安全修复进行同步，不再持续扩展独立业务功能。

---

# Warden Worker

# 有问题？尝试 [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/afoim/warden-worker)

Warden Worker 是一个运行在 Cloudflare Workers 上的轻量级 Bitwarden 兼容服务端实现，使用 Cloudflare D1（SQLite）作为数据存储，核心代码用 Rust 编写，目标是“个人/家庭可用、部署成本低、无需维护服务器”。

本项目不接触你的明文密码：Bitwarden 系列客户端会在本地完成加密，服务端只保存密文数据。

> [!WARNING]
> 如果你曾经部署过旧版本并准备升级，建议在客户端导出密码库 → 重新部署本项目（全新初始化数据库）→ 再导入密码库（可显著降低迁移/兼容成本）。

## 功能

- 无服务器部署：Cloudflare Workers + D1
- 兼容多端：官方 Bitwarden（浏览器扩展 / 桌面 / 安卓）与多数第三方客户端
- 核心能力：注册/登录、同步、密码项（Cipher）增删改、文件夹、TOTP（Authenticator）二步验证
- 官方安卓兼容：支持 `/api/devices/knowndevice` 与 remember-device（twoFactorProvider=5）流程

## 客户端使用建议

- 官方安卓如果之前指向过其它自托管地址，建议“删除账号/清缓存后重新添加服务器”，避免 remember token 跨服务端复用导致登录失败。
- 首次启用 TOTP 后，建议在同一台设备上完成一次“输入 TOTP 登录”，后续官方安卓会自动走 remember-device（provider=5）。

## 已实现的关键接口（部分）

- 配置与探测：`GET /api/config`、`GET /api/alive`、`GET /api/now`、`GET /api/version`
- 登录：`POST /identity/accounts/prelogin`、`POST /identity/connect/token`
- 同步：`GET /api/sync`
- 密码项：`POST /api/ciphers/create`、`PUT /api/ciphers/{id}`、`PUT /api/ciphers/{id}/delete`
- 文件夹：`POST /api/folders`、`PUT /api/folders/{id}`、`DELETE /api/folders/{id}`
- 2FA：`GET /api/two-factor`、`/api/two-factor/authenticator/*`
- 官方安卓设备探测：`GET /api/devices/knowndevice`

## 完整 Cloudflare 部署流程

以下流程适用于本地手动部署。推荐的测试/生产自动发布流程见第 8 节；自动发布会使用 GitHub Environment 中的 D1 和 Worker 配置，不依赖仓库内固定的账户、D1 或 Worker 名称。

### 1. 安装和登录

```bash
npm install --global wrangler
cargo install worker-build --locked
rustup target add wasm32-unknown-unknown
wrangler login
wrangler whoami
```

部署 `HeavyDo` 需要 Workers、D1 和 Durable Objects 权限；建议确认账号套餐支持 Durable Objects。

### 2. 创建 D1 并记录配置

```bash
wrangler d1 create warden-sql
```

记录输出的 `database_id` 和数据库名称。手动部署时写入对应 Wrangler 配置；GitHub Actions 部署时将它们分别填入 Environment Variables `D1_DATABASE_ID` 和 `D1_DATABASE_NAME`。

### 3. 初始化或升级数据库

仅对全新空数据库执行初始化；`schema_full.sql` 可能删除已有表，已有数据时禁止执行：

```bash
wrangler d1 execute vault1 --remote \
  --config wrangler.production.jsonc --file sql/schema_full.sql
```

已有环境只能使用增量迁移：

```bash
wrangler d1 migrations apply vault1 --remote \
  --config wrangler.production.jsonc
```

注册白名单表由增量 migration 创建。`HeavyDo` 的 Durable Object migration 由 `wrangler deploy` 自动根据 migration tag 处理，不需要手动执行 SQL。

### 4. 配置管理员和 Secrets

这些配置是 Cloudflare Worker 的运行时 Secrets，不会参与 Rust/WASM 构建。因此可以在本地构建之前或之后设置，但必须在第一次部署 Worker 之前完成；GitHub Actions 不会从代码仓库自动创建这些值。

生产环境设置：

```bash
wrangler secret put JWT_SECRET --config wrangler.production.jsonc
wrangler secret put JWT_REFRESH_SECRET --config wrangler.production.jsonc
wrangler secret put ADMIN_EMAIL --config wrangler.production.jsonc
wrangler secret put SIGNUPS_ALLOWED --config wrangler.production.jsonc
wrangler secret put TWO_FACTOR_ENC_KEY --config wrangler.production.jsonc
```

`JWT_SECRET`、`JWT_REFRESH_SECRET` 和 `TWO_FACTOR_ENC_KEY` 必须使用高强度随机值：

```bash
openssl rand -base64 48
openssl rand -base64 48
openssl rand -base64 32
```

`ADMIN_EMAIL` 必须对应一个已注册用户；它不是自动创建的管理员账号。首次部署建议将 `SIGNUPS_ALLOWED=true`，先注册管理员，再访问 `https://你的域名/admin.html` 验证后台，之后按需改为 `false` 关闭注册。

配置时机和用途：

| 配置 | 设置时机 | 说明 |
|---|---|---|
| `JWT_SECRET` | 首次部署前 | 访问 JWT 签名密钥，生产环境必需 |
| `JWT_REFRESH_SECRET` | 首次部署前 | refresh token 签名密钥，生产环境必需 |
| `ADMIN_EMAIL` | 首次部署前 | 管理员邮箱；对应账号需要先注册 |
| `SIGNUPS_ALLOWED` | 首次部署前 | `true` 开放注册，`false` 关闭注册 |
| `TWO_FACTOR_ENC_KEY` | 启用 TOTP 前 | 建议首次部署前配置，避免后续更换加密方式 |

如果已经完成构建但还没有设置 Secrets，可以直接设置后再执行部署，不需要重新构建：

```bash
wrangler secret put ADMIN_EMAIL --config wrangler.production.jsonc
wrangler secret put SIGNUPS_ALLOWED --config wrangler.production.jsonc
wrangler deploy --config wrangler.production.jsonc
```

### 5. 注册邮箱白名单

注册总开关由 `SIGNUPS_ALLOWED` 控制，`registration_allowlist` 表决定细粒度权限：

- 表为空：允许所有邮箱注册。
- 表中有记录：仅允许 `enabled=1` 的邮箱注册。
- 禁用邮箱不会删除已有账号。
- 删除最后一条记录后，白名单为空，注册恢复公开（前提是 `SIGNUPS_ALLOWED=true`）。

管理页面：`https://你的域名/admin.html`。管理员 API 要求 `Authorization: Bearer <access_token>`，仅 Cookie 登录不能通过管理员校验。

```text
GET    /api/admin/summary
GET    /api/admin/users
GET    /api/admin/allowlist
POST   /api/admin/allowlist
PATCH  /api/admin/allowlist/{email}
DELETE /api/admin/allowlist/{email}
```

添加白名单：

```bash
curl -X POST https://你的域名/api/admin/allowlist \
  -H 'Authorization: Bearer <管理员访问令牌>' \
  -H 'Content-Type: application/json' \
  --data '{"email":"user@example.com","enabled":true}'
```

暂停邮箱：

```bash
curl -X PATCH 'https://你的域名/api/admin/allowlist/user%40example.com' \
  -H 'Authorization: Bearer <管理员访问令牌>' \
  -H 'Content-Type: application/json' \
  --data '{"enabled":false}'
```

### 6. 部署 Worker

```bash
wrangler deploy --config wrangler.production.jsonc
```

部署输出必须包含 `env.HEAVY_DO (HeavyDo) Durable Object` 和 `env.vault1 (...) D1 Database`。首次部署会登记 `HeavyDo`；以后部署由 Wrangler 根据 migration tag 自动判断是否需要执行 DO migration，不要修改已经发布的 tag。

### 7. 验证部署

```bash
./scripts/smoke-test.sh https://你的域名
curl -f https://你的域名/api/alive
curl -f https://你的域名/api/version
curl -f https://你的域名/api/config
```

还应验证 `/demo.html`、`/admin.html`、新旧账号登录、refresh token、同步、文本/文件 Send、TOTP 和 Android remember-device。

### 8. GitHub Actions 自动部署

仓库提供四个 workflow：`Deploy test`（仅手动触发，读取 `test` Environment）、`Deploy production`（推送 `main` 或手动触发，读取 `production` Environment）、`Initialize test D1` 和 `Initialize production D1`（仅手动触发）。测试流程不会因代码推送自动运行，也不会读取或修改生产 Environment。

#### 第一次配置顺序

1. 在 Cloudflare 创建独立的测试/生产 D1，记录各自的数据库名称和 ID。
2. 创建 API Token，并将 Account Resources 限定到目标账户。至少授予账户级 `Workers Scripts: Edit`、`D1: Edit`、`Account Settings: Read`；不要使用 Global API Key。
3. 在 GitHub `Settings → Environments` 创建 `test`、`production`。生产环境建议设置 Required reviewers。
4. 每个 Environment 的 **Variables** 配置：`WORKER_NAME`（默认生产 `warden-worker`、测试 `warden-worker-test`）、`D1_DATABASE_NAME`、`D1_DATABASE_ID`、`ADMIN_EMAIL`、`SIGNUPS_ALLOWED`、`BASE_URL`。
5. 每个 Environment 的 **Secrets** 配置：`CLOUDFLARE_API_TOKEN`、`CLOUDFLARE_ACCOUNT_ID`、`WORKER_JWT_SECRET`、`WORKER_JWT_REFRESH_SECRET`、`WORKER_TWO_FACTOR_ENC_KEY`。
6. 第一次只对空数据库手动运行对应的 `Initialize ... D1`；workflow 会先只读检查核心表，检测到任意数据就直接退出，不执行 `sql/schema_full.sql`。该 SQL 仍可能清空表，已有数据禁止运行。
7. 首次部署测试时，在 Actions 页面手动运行 `Deploy test` 并选择要验证的 ref；生产推送 `main` 或手动运行 `Deploy production`。
8. 如使用自定义域，在 Worker 部署后到 Cloudflare `Workers → Settings → Domains & Routes → Add Custom Domain` 绑定，再把该 URL 写入 `BASE_URL`。

Workflow 每次按此顺序执行：检出代码 → 构建 Rust/WASM → 校验版本 → 用 Variables 生成 `/tmp/wrangler.deploy.jsonc` → 应用增量 D1 migration → 用 `wrangler secret put` 把 GitHub Secrets/Variables 同步为 Worker 运行时 Secrets → 部署 Worker/ Durable Object → 执行可选 smoke test。

注意：`CLOUDFLARE_API_TOKEN` 只是 GitHub Actions 调用 Cloudflare API 的凭据，不会自动成为 Worker Secret；运行时的 JWT、TOTP、管理员和注册开关由 workflow 显式同步。D1 名称、D1 ID、Worker 名称不会再硬编码到自动部署配置；模板见 `wrangler.deploy.template.jsonc`，渲染脚本见 `scripts/render-wrangler-config.mjs`。

### 9. 备份、升级和回滚

已有环境升级只能执行 `wrangler d1 migrations apply`，绝对不要重新执行 `sql/schema_full.sql`。发布前使用 Cloudflare D1 导出能力或受控脚本备份，不要将包含邮箱、密文或 token 的备份提交到 Git。

应用回归时保留已执行的 D1/DO migration，选择上一个已验证的 Git ref 手动触发生产 workflow 重新部署 Worker；不要删除 Durable Object 类或回退数据库结构。修复后再合并到 `main`。

### 10. 本地开发

```bash
wrangler d1 execute vault1 --local --file sql/schema_full.sql
wrangler dev --config wrangler.jsonc
```

本地 Secrets 放在未提交的 `.dev.vars`，生产 Secrets 不要复制进去：

```text
JWT_SECRET=local-development-secret
JWT_REFRESH_SECRET=local-development-refresh-secret
SIGNUPS_ALLOWED=true
ADMIN_EMAIL=admin@example.com
TWO_FACTOR_ENC_KEY=<base64-32-byte-key>
```

## 常见问题

### 登录返回 400

检查 `grant_type`、refresh token 是否过期，以及客户端是否连接了正确环境。`/identity/connect/token` 支持 password、refresh_token 和 send_access 流程。

### 发送文本失败

客户端先通过 `grant_type=send_access` 获取短期 token，再使用 Bearer token 调用 `/api/sends/access`；普通用户 JWT 不能代替 Send access token。

### 注册被拒绝

检查 `SIGNUPS_ALLOWED`、`registration_allowlist` 是否为空、邮箱是否为 `enabled=1`，并确认客户端连接的是目标环境。

### 管理页面无法操作

确认当前登录邮箱与 `ADMIN_EMAIL` 一致，并使用 `Authorization: Bearer`；仅 Cookie 登录不会通过管理 API。

## 许可证

MIT

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=afoim/warden-worker&type=date&legend=top-left)](https://www.star-history.com/#afoim/warden-worker&type=date&legend=top-left)
