# Vaultwarden 上游变更摘要（2026-11-04）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`6ed6921cc2` → `17b159beff`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/6ed6921cc2b0e03b4d74189574d2a5e71c512e97...17b159beff4b892f07f61f38ce2fe452e55251c1)）
- 提交数：4；变更文件数：21
- 生成时间：2026-11-09 09:43 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`28649978`](https://github.com/dani-garcia/vaultwarden/commit/286499781ad7c8e2a3c173ced6f4e0a1fa28b8ad) Add Fill Assist feature flag in .env.template (#7928)
- [`82fc47e8`](https://github.com/dani-garcia/vaultwarden/commit/82fc47e83e39c5d09e85a6f259c8ab51166735d1) Redact query values in request logs (#7929)
- [`904b8011`](https://github.com/dani-garcia/vaultwarden/commit/904b80115d774fe3238408c19303720ef6def2b2) [PR 1] Add experimental Custom Roles foundation (#7847)
- [`17b159be`](https://github.com/dani-garcia/vaultwarden/commit/17b159beff4b892f07f61f38ce2fe452e55251c1) Don't let OpenDAL create a missing config folder (#7935)

## 变更文件（前 200 个）

- modified `.env.template` (+10/-0)
- added `migrations/mysql/2026-10-10-120000_add_custom_role_permissions/down.sql` (+22/-0)
- added `migrations/mysql/2026-10-10-120000_add_custom_role_permissions/up.sql` (+13/-0)
- added `migrations/postgresql/2026-10-10-120000_add_custom_role_permissions/down.sql` (+20/-0)
- added `migrations/postgresql/2026-10-10-120000_add_custom_role_permissions/up.sql` (+13/-0)
- added `migrations/sqlite/2026-10-10-120000_add_custom_role_permissions/down.sql` (+19/-0)
- added `migrations/sqlite/2026-10-10-120000_add_custom_role_permissions/up.sql` (+11/-0)
- modified `src/api/admin.rs` (+32/-6)
- modified `src/api/core/organizations.rs` (+38/-18)
- modified `src/api/web.rs` (+2/-1)
- modified `src/config.rs` (+8/-0)
- modified `src/db/mod.rs` (+71/-0)
- modified `src/db/models/cipher.rs` (+6/-6)
- modified `src/db/models/collection.rs` (+7/-7)
- modified `src/db/models/mod.rs` (+2/-2)
- modified `src/db/models/organization.rs` (+319/-24)
- modified `src/db/schema.rs` (+9/-0)
- modified `src/static/scripts/admin_users.js` (+12/-4)
- modified `src/static/templates/admin/users.hbs` (+5/-0)
- modified `src/static/templates/scss/vaultwarden.scss.hbs` (+7/-0)
- modified `src/util.rs` (+42/-2)

## 可能影响协议/客户端兼容性的文件

- `migrations/mysql/2026-10-10-120000_add_custom_role_permissions/down.sql`
- `migrations/mysql/2026-10-10-120000_add_custom_role_permissions/up.sql`
- `migrations/postgresql/2026-10-10-120000_add_custom_role_permissions/down.sql`
- `migrations/postgresql/2026-10-10-120000_add_custom_role_permissions/up.sql`
- `migrations/sqlite/2026-10-10-120000_add_custom_role_permissions/down.sql`
- `migrations/sqlite/2026-10-10-120000_add_custom_role_permissions/up.sql`
- `src/api/admin.rs`
- `src/api/core/organizations.rs`
- `src/api/web.rs`
- `src/db/mod.rs`
- `src/db/models/cipher.rs`
- `src/db/models/collection.rs`
- `src/db/models/mod.rs`
- `src/db/models/organization.rs`
- `src/db/schema.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
