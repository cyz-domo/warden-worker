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
- 通行密钥（WebAuthn/Passkey）：支持使用通行密钥登录，以及把安全密钥作为二步验证（twoFactorProvider=7）
- 官方安卓兼容：支持 `/api/devices/knowndevice` 与 remember-device（twoFactorProvider=5）流程

## 客户端使用建议

- 官方安卓如果之前指向过其它自托管地址，建议“删除账号/清缓存后重新添加服务器”，避免 remember token 跨服务端复用导致登录失败。
- 首次启用 TOTP 后，建议在同一台设备上完成一次“输入 TOTP 登录”，后续官方安卓会自动走 remember-device（provider=5）。

## 已实现的关键接口（部分）

- 配置与探测：`GET /api/config`、`GET /api/alive`、`GET /api/now`、`GET /api/version`
- 登录：`POST /identity/accounts/prelogin`、`POST /identity/connect/token`
- 同步：`GET /api/sync`
- 密码项：`POST /api/ciphers/create`、`GET /api/ciphers/{id}`、`PUT /api/ciphers/{id}`、`PUT /api/ciphers/{id}/partial`、`PUT /api/ciphers/{id}/delete`
- 文件夹：`POST /api/folders`、`PUT /api/folders/{id}`、`DELETE /api/folders/{id}`
- 2FA：`GET /api/two-factor`、`/api/two-factor/authenticator/*`、`/api/two-factor/webauthn`、`/api/two-factor/get-webauthn*`
- 通行密钥：`GET /identity/accounts/webauthn/assertion-options`、`GET|POST|PUT /api/webauthn`、`POST /api/webauthn/attestation-options`
- 官方安卓设备探测：`GET /api/devices/knowndevice`

## 完整 Cloudflare 部署流程

以下流程适用于本地手动部署。推荐的测试/生产自动发布流程见第 8 节；自动发布会使用 GitHub Environment 中的 D1 和 Worker 配置，不依赖仓库内固定的账户、D1 或 Worker 名称。

### 1. 安装和登录

```bash
npm install --global wrangler
cargo install worker-build@0.8.7 --locked
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
wrangler secret put BACKUP_ENC_KEY --config wrangler.production.jsonc
```

`JWT_SECRET`、`JWT_REFRESH_SECRET`、`TWO_FACTOR_ENC_KEY` 和 `BACKUP_ENC_KEY` 必须使用高强度随机值：

```bash
openssl rand -base64 48
openssl rand -base64 48
openssl rand -base64 32
openssl rand -base64 32
```

`ADMIN_EMAIL` 必须对应一个已注册用户；它不是自动创建的管理员账号。首次部署建议将 `SIGNUPS_ALLOWED=true`，先注册管理员，再访问 `https://你的域名/admin` 验证后台，之后按需改为 `false` 关闭注册。

配置时机和用途：

| 配置 | 设置时机 | 说明 |
|---|---|---|
| `JWT_SECRET` | 首次部署前 | 访问 JWT 签名密钥，生产环境必需 |
| `JWT_REFRESH_SECRET` | 首次部署前 | refresh token 签名密钥，生产环境必需 |
| `ADMIN_EMAIL` | 首次部署前 | 管理员邮箱；对应账号需要先注册 |
| `SIGNUPS_ALLOWED` | 首次部署前 | `true` 开放注册，`false` 关闭注册 |
| `TWO_FACTOR_ENC_KEY` | 启用 TOTP 前 | 建议首次部署前配置，避免后续更换加密方式 |
| `BACKUP_ENC_KEY` | 使用 WebDAV 备份前 | 32 字节 base64 密钥，加密备份文件与 WebDAV 密码；未配置时备份功能不可用 |
| `TWO_FACTOR_REMEMBER_DAYS` | 可选 | remember-device（provider=5）记住登录的有效天数，取值 1 到 90，未设置时按 30 天，与上游 `TWO_FACTOR_REMEMBER_DAYS` 同名同义 |
| `WEBAUTHN_RP_ID` | 可选 | 通行密钥的 RP ID，默认为当前请求域名；当 Worker 通过自定义域访问而通行密钥需要绑定根域时手动指定 |
| `WEBAUTHN_ORIGIN` | 可选 | 校验 `clientData.origin` 的完整 origin（含协议和端口），默认为当前请求的 origin；前后端同域时通常无需设置 |

通行密钥说明：

- 注册和登录都要求验证器支持 ES256（P-256），并会校验 RP ID、origin、用户存在/验证标志和签名计数器。
- 通行密钥登录要求使用可发现凭据（discoverable credential / resident key），因此注册时会要求 `residentKey=required`；仅支持非驻留凭据的安全密钥只能用于二步验证。
- 目前不支持 PRF：密钥列表中的 `PrfStatus` 固定为 2（Unsupported），因此「使用通行密钥解锁密码库」不可用，但通行密钥登录和二步验证不受影响。
- `workers.dev` 域名的 RP ID 是 `*.workers.dev` 的子域，部分平台（尤其 iOS/macOS）要求 RP ID 与可注册域后缀匹配。如果遇到注册失败，请绑定自定义域并设置 `WEBAUTHN_RP_ID` 与 `WEBAUTHN_ORIGIN`。
- WebAuthn 表由 Worker 运行时自动创建；已有部署无需手动迁移。二步验证密钥（TOTP）仍只使用 `TWO_FACTOR_ENC_KEY`，WebAuthn 公钥和挑战不经过该密钥加密。

已有部署不需要额外的迁移步骤：Worker 会在首次使用 remember-device 时自动为 `devices` 表补上 `remember_expires_at` 列（与 `remember_token_hash` 相同的处理方式），全新安装则由 `sql/schema_full.sql` 建表时带上该列。改动之前签发的 remember token 没有记录过期时间，会被视为已过期：相关设备下一次登录需要重新输入一次 TOTP，之后按新的有效期记住。

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

管理页面：`https://你的域名/admin`（与上游 Vaultwarden 把后台挂在 `/admin` 的习惯一致），`/admin.html` 仍然可用。页面本身由 Worker 路由返回，需要先用 web-vault 登录（依赖 session cookie）；管理员 API 要求 `Authorization: Bearer <access_token>`，仅 session cookie 不能通过管理员校验。

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

还应验证 `/demo.html`、`/admin`、`/admin.html`、新旧账号登录、refresh token、同步、文本/文件 Send、TOTP、Android remember-device，以及通行密钥注册/登录和 WebAuthn 二步验证。

### 8. GitHub Actions 自动部署

仓库提供五个主要 workflow：`Deploy test`（仅手动触发，读取 `test` Environment）、`Deploy production`（推送 `main` 或手动触发，读取 `production` Environment）、`Initialize test D1`、`Initialize production D1`（仅手动触发）以及 `Sync Vaultwarden upstream`（定时检查上游并自动生成变更摘要 PR）。测试流程不会因代码推送自动运行，也不会读取或修改生产 Environment。

#### 第一次配置顺序

1. 在 Cloudflare 创建独立的测试/生产 D1，记录各自的数据库名称和 ID。
2. 创建 API Token，并将 Account Resources 限定到目标账户。至少授予账户级 `Workers Scripts: Edit`、`D1: Edit`、`Account Settings: Read`；不要使用 Global API Key。
3. 在 GitHub `Settings → Environments` 创建 `test`、`production`。生产环境建议设置 Required reviewers。
4. 每个 Environment 的 **Variables** 配置：`WORKER_NAME`（默认生产 `warden-worker`、测试 `warden-worker-test`）、`D1_DATABASE_NAME`、`D1_DATABASE_ID`、`ADMIN_EMAIL`、`SIGNUPS_ALLOWED`、`BASE_URL`。
5. 每个 Environment 的 **Secrets** 配置：`CLOUDFLARE_API_TOKEN`、`CLOUDFLARE_ACCOUNT_ID`、`WORKER_JWT_SECRET`、`WORKER_JWT_REFRESH_SECRET`、`WORKER_TWO_FACTOR_ENC_KEY`；如需 WebDAV 备份，再配置 `WORKER_BACKUP_ENC_KEY`（可选，未配置时部署不会失败，但备份功能保持禁用）。

> [!IMPORTANT]
> **请务必把 `WORKER_BACKUP_ENC_KEY` 和 `WORKER_TWO_FACTOR_ENC_KEY` 写入 GitHub Environment Secrets，并另外离线保存一份。** 这两个密钥只以 write-only Secret 的形式存在于 GitHub 和 Worker 上，部署日志与仓库都不会保存明文。`WORKER_BACKUP_ENC_KEY` 丢失后旧备份和已保存的 WebDAV 密码无法解密；`WORKER_TWO_FACTOR_ENC_KEY` 丢失后已绑定的 TOTP 验证码会全部失效。不要为了“清理”而删除这两个 Secret，也不要在不同部署之间随意轮换。
6. 第一次只对空数据库手动运行对应的 `Initialize ... D1`；workflow 会先只读检查核心表，检测到任意数据就直接退出，不执行 `sql/schema_full.sql`。该 SQL 仍可能清空表，已有数据禁止运行。
7. 首次部署测试时，在 Actions 页面手动运行 `Deploy test` 并选择要验证的 ref；生产推送 `main` 或手动运行 `Deploy production`。
8. 如使用自定义域，在 Worker 部署后到 Cloudflare `Workers → Settings → Domains & Routes → Add Custom Domain` 绑定，再把该 URL 写入 `BASE_URL`。

Workflow 每次按此顺序执行：检出代码 → 构建 Rust/WASM → 校验版本 → 用 Variables 在仓库根目录生成临时 Wrangler 配置 → 应用增量 D1 migration → 用 `wrangler secret put` 把 GitHub Secrets/Variables 同步为 Worker 运行时 Secrets → 部署 Worker/ Durable Object → 执行可选 smoke test。

注意：`CLOUDFLARE_API_TOKEN` 只是 GitHub Actions 调用 Cloudflare API 的凭据，不会自动成为 Worker Secret；运行时的 JWT、TOTP、管理员和注册开关由 workflow 显式同步。D1 名称、D1 ID、Worker 名称不会再硬编码到自动部署配置；模板见 `wrangler.deploy.template.jsonc`，渲染脚本见 `scripts/render-wrangler-config.mjs`。

### 8.1 Vaultwarden 上游兼容同步

`.github/workflows/upstream-vaultwarden-sync.yml`（`Sync Vaultwarden upstream`）每周一检查 `dani-garcia/vaultwarden` 的 `main` 分支，也支持在 Actions 页面手动运行。发现新提交后，它会：

1. 自动生成「变更摘要 PR」：列出对比区间、提交列表、变更文件，并单独标注可能影响 Bitwarden 协议/客户端兼容性的文件；摘要文档提交在 `docs/upstream/vaultwarden-<日期>.md`。
2. 创建/更新「移植任务 Issue」（`[Upstream sync] Port Vaultwarden changes <日期>`），引用摘要 PR 并自动分配给 GitHub Copilot Coding Agent。
3. 把检查点推进到已检查到的上游 commit（存于仓库 `upstream-checkpoint` 分支的 `checkpoint.sha`），下次运行从该位置继续对比。

Copilot 收到任务后会分析摘要 PR，只把适用于 Cloudflare Worker 的 API、协议和数据模型改动移植成独立代码 PR，并保留 D1、Durable Objects、KDF、Send、2FA、白名单及部署逻辑；需要 schema 变化时补充 D1 migration。摘要 PR 本身只含文档，不会被合并代码。

合并策略：Copilot 的代码 PR 不会自动合入 `main`。它必须通过仓库现有检查（`Deploy test` / smoke test）并由人工 review 后合并；如需启用自动合并，请先配置 `main` 分支保护（required checks + review），再对代码 PR 开启 auto-merge。

运行前提：仓库 `Settings → Actions → General` 允许 Actions 创建 Issue/PR，并启用 Issues 和 Copilot Coding Agent（若自动分配不可用，需在 Issue 页面手动分配）。首次运行只建立检查点基线、不生成 PR；第二次起每周一自动执行上述流程。

### 9. 备份、升级和回滚

已有环境升级只能执行 `wrangler d1 migrations apply`，绝对不要重新执行 `sql/schema_full.sql`。发布前使用 Cloudflare D1 导出能力或受控脚本备份，不要将包含邮箱、密文或 token 的备份提交到 Git。

应用回归时保留已执行的 D1/DO migration，选择上一个已验证的 Git ref 手动触发生产 workflow 重新部署 Worker；不要删除 Durable Object 类或回退数据库结构。修复后再合并到 `main`。

### 9.1 WebDAV 定时备份与恢复（`/admin`）

配置 `BACKUP_ENC_KEY` 后，`/admin` 页面会出现「加密备份（WebDAV）」区域：

- 通道支持 https WebDAV 目录、Basic 认证、每天/每周计划、UTC 偏移（默认 UTC+08:00，不处理夏令时）和保留份数（1–100）。
- Worker 通过每 15 分钟一次的 Cron Trigger 检查到期通道，也可以在后台点「立即备份」。
- 备份在 Worker 内导出 `users`、`folders`、`ciphers`、`sends`、`send_files`、`send_file_chunks`、`two_factor_authenticator`、`devices`、`registration_allowlist` 九张表，整体用 AES-256-GCM 加密后以 `warden-<UTC时间>-<随机>.enc` 上传；WebDAV 密码同样以该密钥加密存储。
- 恢复会先上传 `warden-pre-restore-*.enc` 当前状态快照，再以单事务覆盖上述表；任何一步失败都会回滚。恢复后 `security_stamp` 回到备份时的值，所有现有登录会话失效，需要重新登录。
- 限制：明文 JSON 16 MiB、总行数 20000、单文件 64 MiB。超过限制请改用 `wrangler d1 export` 在本地备份；恢复前请确认远程目录中已有完整备份。
- `BACKUP_ENC_KEY` 丢失后无法解密旧备份或已保存的 WebDAV 密码，请备份并妥善保管该密钥，不要随意轮换。
- 备份内含 TOTP 密钥与用户数据；跨部署恢复时需要同时使用相同的 `BACKUP_ENC_KEY` 和 `TWO_FACTOR_ENC_KEY`，否则备份无法解密或 TOTP 无法继续使用。
- 恢复入口：后台对应通道行点「文件」，在远程备份列表中点「恢复」，需要二次确认。
- 二步验证（TOTP）密钥用 `TWO_FACTOR_ENC_KEY` 单独加密，和备份文件的 `BACKUP_ENC_KEY` 无关。如果备份来自另一个部署、两边的 `TWO_FACTOR_ENC_KEY` 不同，恢复后所有账户的 App 验证码都会校验失败。此时在「恢复与导入」勾选「恢复后清除所有账户的二步验证（App 验证码）」再执行恢复：会删除全部验证器绑定和恢复码，并撤销记住设备，各账户用密码登录后重新绑定即可。
- 离线导入：后台「恢复与导入」支持直接上传本地 `warden-*.enc` 文件（上限 24 MiB），用当前 `BACKUP_ENC_KEY` 解密后覆盖恢复，并记入运行记录。离线导入没有可用的 WebDAV 快照位置，不会自动上传恢复前快照，建议先「立即备份」当前状态；同样支持上述「清除二步验证」选项。
- 后台页面采用左侧菜单布局：概览、注册白名单、备份通道、运行记录、恢复与导入、用户。

### 9.2 在全新 Worker/D1 上恢复

备份文件与 Worker 实例无关，只要手里有 `warden-*.enc` 和原来的 `BACKUP_ENC_KEY`，就可以在一个全新的 Worker + D1 上把数据恢复出来：

1. 新建 D1，并初始化空库：已有数据库执行 `wrangler d1 migrations apply`，全新空库可执行 `sql/schema_full.sql`。
2. 配置运行时 Secrets：`BACKUP_ENC_KEY` 和 `TWO_FACTOR_ENC_KEY` 必须与备份时一致；`JWT_SECRET`/`JWT_REFRESH_SECRET` 可以是新值（旧会话本来就会失效）；`ADMIN_EMAIL` 保持备份中的管理员邮箱。
3. 临时设置 `SIGNUPS_ALLOWED=true` 并部署，然后用该管理员邮箱注册一个临时账号并登录 `/admin`——恢复 API 要求管理员 Bearer token，空库需要这个跳板账号。
4. 在「加密备份（WebDAV）」中重新添加一个指向原备份目录的通道（通道配置不在备份内），点「测试」确认连接正常。
5. 点该通道的「文件」，选择目标 `warden-*.enc` 并「恢复」。如果新实例的 `TWO_FACTOR_ENC_KEY` 与备份来源不同，先勾选「恢复后清除所有账户的二步验证（App 验证码）」。恢复会先上传 `warden-pre-restore-*.enc` 快照，再整体覆盖数据表；完成后所有会话失效。
6. 用备份时的原始账号重新登录，确认密码项、文件夹、Send、设备、TOTP 状态和注册白名单均已恢复；最后把 `SIGNUPS_ALLOWED` 改回 `false`。如果第 5 步清除了二步验证，登录后到安全设置里重新绑定验证器 App。

注意事项：

- 恢复只覆盖 `users`、`folders`、`ciphers`、`sends`、`send_files`、`send_file_chunks`、`two_factor_authenticator`、`devices`、`registration_allowlist`；不包含 `backup_channels`、`backup_runs`、`backup_locks`，新实例中刚添加的 WebDAV 通道会保留，可继续用于定时备份。
- 空库里临时注册的管理员账号会在恢复时被备份中的原始用户数据替换，因此请确认记得原始账号的主密码。
- 如果旧实例仍然在线且指向同一个 WebDAV 目录，恢复本身不受影响，但两边的定时任务可能互相清理旧文件；切换完成后请关闭旧实例的通道或停用旧 Worker。
- 如果还能拿到旧实例的管理员 access token，也可以跳过临时注册，直接调用 `POST /api/admin/backup-channels/{id}/restore`（body 为 `{"file":"warden-....enc","confirm":true}`）。
- 请使用当前或更新版本的 Worker 恢复旧备份；如果备份文件缺少当前代码期望的列，恢复会明确报错而不会写入不完整数据。

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
BACKUP_ENC_KEY=<base64-32-byte-key>
```

## 常见问题

### 登录返回 400

检查 `grant_type`、refresh token 是否过期，以及客户端是否连接了正确环境。`/identity/connect/token` 支持 password、refresh_token 和 send_access 流程。

### prelogin 返回 429

`/identity/accounts/prelogin` 与 `/api/accounts/prelogin`（含 `/password` 变体）在未登录情况下会返回账号真实的 KDF 参数，因此按客户端地址计数：每个地址每 60 秒 30 次，超出返回 429 并带 `Retry-After: 60`，与上游 #7681 的思路一致。计数保存在处理请求的 isolate 内存里（`prelogin` 走 HeavyDo，`/password` 变体走 Worker），isolate 回收后重新起算，属于尽力而为的限制；`/identity/connect/token` 不受该计数影响。

### 发送文本失败

客户端先通过 `grant_type=send_access` 获取短期 token，再使用 Bearer token 调用 `/api/sends/access`；普通用户 JWT 不能代替 Send access token。

### 注册被拒绝

检查 `SIGNUPS_ALLOWED`、`registration_allowlist` 是否为空、邮箱是否为 `enabled=1`，并确认客户端连接的是目标环境。

### 管理页面无法操作

确认当前登录邮箱与 `ADMIN_EMAIL` 一致，并使用 `Authorization: Bearer`；仅 session cookie 登录不会通过管理 API。后台入口是 `/admin`（`/admin.html` 同样可用）；访问时返回 401 表示当前 session cookie 不属于 `ADMIN_EMAIL` 对应的账号。

### 恢复后输入 App 验证码一直转圈或提示无效

二步验证密钥用 `TWO_FACTOR_ENC_KEY` 单独加密保存。恢复/导入只保证 `BACKUP_ENC_KEY` 一致，不会校验 `TWO_FACTOR_ENC_KEY`：两边密钥不同时，服务端无法解密备份里的 TOTP 密钥，验证码自然全部无效（日志里会出现 `Authenticator secret cannot be decrypted`）。

处理方式二选一：

- 如果还保留启用二步验证时的恢复码，选择「使用恢复码」登录（接口为 `twoFactorProvider=8`），登录成功后该账户的验证器会自动停用，再重新绑定。
- 否则重新执行一次恢复，并在「恢复与导入」勾选「恢复后清除所有账户的二步验证（App 验证码）」；恢复后所有账户用密码登录并重新绑定。demo 环境也可以手动运行 `Deploy demo` 并勾选 `clear_two_factor` 输入项直接清除。
- 已经恢复完成、所有人都被挡在 2FA 之外时，可以直接清空 D1 中的二步验证状态（会同时撤销记住设备）：

```bash
wrangler d1 execute vault1 --remote --config wrangler.production.jsonc --command \
  "DELETE FROM two_factor_authenticator; DELETE FROM two_factor_webauthn; DELETE FROM webauthn_challenges; UPDATE users SET totp_recover = NULL; UPDATE devices SET remember_token_hash = NULL, remember_expires_at = NULL;"
```

跨部署恢复时请保证 `TWO_FACTOR_ENC_KEY` 与备份来源一致，或提前勾选清除，可避免此问题。

### 登录返回 405，但无痕窗口可以

空 body 的 405 一般说明请求没有进入 Worker，而是被 Cloudflare 静态资源层挡掉了：客户端把带子路径的地址（例如 `https://你的域名/admin`）当作 environment base URL 记在了该站点的应用存储里，之后每个 API 请求都变成 `/admin/identity/...`。Worker 现在接管了 `/admin`，并把 `/admin/{api,identity,sends}/*` 去掉前缀后按正常路由处理，所以这类历史数据可以自愈。若仍有异常，请在该站点的 DevTools → Application 中删除对应的应用存储记录，并从根路径 `https://你的域名/` 重新打开。

## 许可证

MIT

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=afoim/warden-worker&type=date&legend=top-left)](https://www.star-history.com/#afoim/warden-worker&type=date&legend=top-left)

