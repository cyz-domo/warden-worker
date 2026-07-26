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

以下流程适用于生产部署。仓库的自动发布只针对 `main` 分支；`wrangler.test.jsonc` 仅保留作本地或历史环境配置，不参与自动发布。

### 1. 安装和登录

```bash
npm install --global wrangler
cargo install worker-build --locked
rustup target add wasm32-unknown-unknown
wrangler login
wrangler whoami
```

部署 `HeavyDo` 需要 Workers、D1 和 Durable Objects 权限；建议确认账号套餐支持 Durable Objects。

### 2. 创建 D1 并写入配置

```bash
wrangler d1 create warden-sql
```

将输出的生产 `database_id` 写入 `wrangler.production.jsonc`，并保留 `binding: "vault1"`、正确的 `database_name` 和 `migrations_dir: "sql/migrations"`。

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

### 8. GitHub 自动部署

可以直接连接 GitHub 仓库自动部署，不需要额外服务器或 GitHub App。仓库的 `.github/workflows/deploy-production.yml` 已配置为：推送 `main` 自动构建、应用 D1 migration、部署 Worker、执行 smoke test；也支持 GitHub Actions 页面手动触发。

#### 必须先配置的 Worker Secrets

`ADMIN_EMAIL`、`SIGNUPS_ALLOWED`、`JWT_SECRET`、`JWT_REFRESH_SECRET` 和 `TWO_FACTOR_ENC_KEY` 不是 GitHub Actions 自动生成的变量，必须在 GitHub Actions 第一次生产部署前写入 Cloudflare 生产 Worker 的 Secrets。它们是运行时配置，不要求在 GitHub Actions 构建阶段提供。

在本地配置生产 Worker：

```bash
wrangler secret put ADMIN_EMAIL --config wrangler.production.jsonc
wrangler secret put SIGNUPS_ALLOWED --config wrangler.production.jsonc
```

建议首次上线时设置：

```text
ADMIN_EMAIL=admin@example.com
SIGNUPS_ALLOWED=true
```

然后使用 `admin@example.com` 注册并登录，打开 `/admin.html` 添加允许注册的邮箱。白名单保存于生产 D1 的 `registration_allowlist` 表，不是环境变量。确认管理员和白名单正常后，可以将 `SIGNUPS_ALLOWED` 更新为 `false` 关闭公开注册：

```bash
wrangler secret put SIGNUPS_ALLOWED --config wrangler.production.jsonc
# Wrangler 提示输入时填写 false
```

以下值也必须提前配置在 Cloudflare 生产 Worker 中：

```bash
wrangler secret put JWT_SECRET --config wrangler.production.jsonc
wrangler secret put JWT_REFRESH_SECRET --config wrangler.production.jsonc
wrangler secret put TWO_FACTOR_ENC_KEY --config wrangler.production.jsonc
```

这些 Worker Secrets 与下面的 GitHub Actions Secrets 是两套不同的配置，不能混淆。

截图中如果还看到以下旧变量，可以删除，不需要再设置：

- `ALLOWED_EMAILS`：旧版邮箱配置，当前白名单由 D1 的 `registration_allowlist` 表管理。
- `SIGNUPS_ALLOWLIST_ONLY`：旧版注册开关，当前项目不读取该变量。

当前项目实际使用的是 `ADMIN_EMAIL`、`SIGNUPS_ALLOWED` 和上述 JWT/TOTP Secrets。白名单邮箱通过 `/admin.html` 写入生产 D1，不是 Cloudflare Worker 环境变量。

在 GitHub 仓库的 `Settings → Environments → production` 中创建/选择 `production` Environment，并在 **Environment secrets** 中添加以下机密，不要放在普通的 Variables 中：

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`
- `PRODUCTION_BASE_URL`（可选；设置后自动执行 smoke test）

其中：

- `CLOUDFLARE_API_TOKEN` 必须是该 Cloudflare 账户的 API Token，不能使用其他账户的 Token。
- `CLOUDFLARE_ACCOUNT_ID` 必须填写 `2a863f1907f04c91e31378c111cd4a26` 这类 32 位 Cloudflare **帐户 ID**，不是 Zone ID/区域 ID。
- `wrangler.production.jsonc` 不固定任何账户 ID；workflow 通过 `CLOUDFLARE_ACCOUNT_ID` 环境变量指定目标账户，便于其他人复用仓库。
- 如果把值放在 GitHub 的 `Variables` 而不是 `Secrets`，当前 workflow 不会读取到它们；请移动到 `production` Environment secrets。
- API Token 必须包含以下账户级权限，并且 Token 所属账户必须与 `CLOUDFLARE_ACCOUNT_ID` 一致：
  - `Workers Scripts: Edit`：上传、发布 Worker 和 Durable Object 类，必须有。
  - `D1: Edit`：执行生产数据库 migration，必须有。
  - `Account Settings: Read`：允许 Wrangler 读取账户设置，建议添加。
- `Workers AI` 权限与本项目部署无关，不需要添加。
- 创建 Token 时，`Account Resources` 必须选择目标 Cloudflare 账户，不能只选择其他账户或仅依赖用户级权限。

建议给 `production` Environment 设置审批人或保护规则，让 `main` 推送后先等待人工批准。

自动部署步骤为：

1. 推送代码到 GitHub `main`。
2. GitHub Actions 检出代码并安装 Rust/WASM 工具链。
3. 校验 Web Vault 版本并构建 Worker。
4. 应用 `sql/migrations` 中尚未执行的生产 D1 migration。
5. 执行 `wrangler deploy --config wrangler.production.jsonc`，DO migration 自动处理。
6. 如果设置了 `PRODUCTION_BASE_URL`，执行 `/api/config`、`/api/version` 和 `/api/alive` 检查。

首次上线仍建议先人工执行一次初始化、配置 Secrets 和管理员账号；之后正常合并到 `main` 即可自动部署。

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
