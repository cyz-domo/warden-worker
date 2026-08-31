# Vaultwarden 上游变更摘要（2026-08-29）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`46d71107f5` → `fdc156b247`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/46d71107f5094460dd5ecbe1dbac6e6c71e5189a...fdc156b247846ca73f6aa3e9c676a6f2f57577cf)）
- 提交数：6；变更文件数：12
- 生成时间：2026-08-31 08:36 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`fa2566d1`](https://github.com/dani-garcia/vaultwarden/commit/fa2566d14fc745937ce104011475eca9e6c7a6f6) Fix password change with newer web-vault (#7634)
- [`10e044f5`](https://github.com/dani-garcia/vaultwarden/commit/10e044f563e6224eb0271419f7b7f4140791dc7f) chore: remove duplicate "the" in ciphers.rs comment (#7254)
- [`83724b30`](https://github.com/dani-garcia/vaultwarden/commit/83724b301e3dd63a0e6f1fe4f46a662bead09480) Ignore reset-password auto-enroll when mail is disabled (#7585)
- [`923f5d0b`](https://github.com/dani-garcia/vaultwarden/commit/923f5d0b5eb7e223855031e35ba9606372ff5fa3) Fix migration for MariaDB 12.2.2 (#7265)
- [`2073c030`](https://github.com/dani-garcia/vaultwarden/commit/2073c03092d328e4b5fd19882ecfbe491dc8b5c7) Add SSO_SIGNUPS_ALLOWED (#7272)
- [`fdc156b2`](https://github.com/dani-garcia/vaultwarden/commit/fdc156b247846ca73f6aa3e9c676a6f2f57577cf) log_event take enum parameter not i32 (#7656)

## 变更文件（前 200 个）

- modified `.env.template` (+3/-0)
- modified `migrations/mysql/2024-03-13-170000_sso_users_cascade/up.sql` (+29/-13)
- modified `playwright/docker-compose.yml` (+1/-1)
- modified `src/api/admin.rs` (+3/-3)
- modified `src/api/core/accounts.rs` (+31/-8)
- modified `src/api/core/ciphers.rs` (+11/-19)
- modified `src/api/core/events.rs` (+3/-3)
- modified `src/api/core/organizations.rs` (+23/-23)
- modified `src/api/core/two_factor/mod.rs` (+3/-11)
- modified `src/api/identity.rs` (+27/-1)
- modified `src/config.rs` (+13/-0)
- modified `src/db/models/org_policy.rs` (+7/-0)

## 可能影响协议/客户端兼容性的文件

- `migrations/mysql/2024-03-13-170000_sso_users_cascade/up.sql`
- `src/api/admin.rs`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/events.rs`
- `src/api/core/organizations.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/identity.rs`
- `src/db/models/org_policy.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
