# Vaultwarden 上游变更摘要（2026-09-18）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`eb212e23fa` → `cc67d644f6`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/eb212e23fad88e6136723f43e5b73543fa7026d3...cc67d644f62605cb46f4d16c4a2eed1a861cc8bb)）
- 提交数：8；变更文件数：30
- 生成时间：2026-09-21 07:55 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`fddc8e71`](https://github.com/dani-garcia/vaultwarden/commit/fddc8e71be5904333acd135734cd1e8f4d8f9c20) Support the vault banner policy (#7748)
- [`8719f86f`](https://github.com/dani-garcia/vaultwarden/commit/8719f86fcf7a58a145eb89ae10987ac11f3135b1) Add basic auth response client feature flag (#7745)
- [`9c8aa235`](https://github.com/dani-garcia/vaultwarden/commit/9c8aa2359ff3e38f5b179b68bba34baeb9d72778) User key id (#7693)
- [`c3f54775`](https://github.com/dani-garcia/vaultwarden/commit/c3f5477525aed713ed340358d3ca982309979032) Add accepted organization data to sync response (#7666)
- [`64c56411`](https://github.com/dani-garcia/vaultwarden/commit/64c56411f6a16ac4466c12199587101566cf7efa) Add `pm-32009-new-item-types` feature flag (#7478)
- [`a2468363`](https://github.com/dani-garcia/vaultwarden/commit/a24683636a448949b0bf535fb2c88ec7f3814b2c) Update Crates, GHA and JS (#7751)
- [`33476987`](https://github.com/dani-garcia/vaultwarden/commit/3347698712d3e99e652ad4394ef2fc0ce2800fdd) Add `pm-34171-card-scanner` feature flag (#7477)
- [`cc67d644`](https://github.com/dani-garcia/vaultwarden/commit/cc67d644f62605cb46f4d16c4a2eed1a861cc8bb) set user_created bool for each separate invitation (#7753)

## 变更文件（前 200 个）

- modified `.env.template` (+3/-0)
- modified `.github/workflows/hadolint.yml` (+1/-1)
- modified `.github/workflows/release.yml` (+3/-3)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+135/-136)
- modified `Cargo.toml` (+7/-7)
- modified `macros/Cargo.toml` (+1/-1)
- added `migrations/cockroachdb/2026-09-02-120000_add_key_id/down.sql` (+0/-0)
- added `migrations/cockroachdb/2026-09-02-120000_add_key_id/up.sql` (+1/-0)
- added `migrations/mysql/2026-09-02-120000_add_key_id/down.sql` (+0/-0)
- added `migrations/mysql/2026-09-02-120000_add_key_id/up.sql` (+1/-0)
- added `migrations/postgresql/2026-09-02-120000_add_key_id/down.sql` (+0/-0)
- added `migrations/postgresql/2026-09-02-120000_add_key_id/up.sql` (+1/-0)
- added `migrations/sqlite/2026-09-02-120000_add_key_id/down.sql` (+0/-0)
- added `migrations/sqlite/2026-09-02-120000_add_key_id/up.sql` (+1/-0)
- modified `src/api/core/accounts.rs` (+21/-2)
- modified `src/api/core/ciphers.rs` (+39/-2)
- modified `src/api/core/events.rs` (+20/-0)
- modified `src/api/core/organizations.rs` (+2/-1)
- modified `src/config.rs` (+4/-0)
- modified `src/db/models/event.rs` (+1/-1)
- modified `src/db/models/mod.rs` (+1/-1)
- modified `src/db/models/org_policy.rs` (+21/-0)
- modified `src/db/models/organization.rs` (+15/-0)
- modified `src/db/models/user.rs` (+30/-0)
- modified `src/db/schema.rs` (+1/-0)
- modified `src/static/scripts/datatables.css` (+14/-2)
- modified `src/static/scripts/datatables.js` (+4937/-4927)

## 可能影响协议/客户端兼容性的文件

- `migrations/cockroachdb/2026-09-02-120000_add_key_id/down.sql`
- `migrations/cockroachdb/2026-09-02-120000_add_key_id/up.sql`
- `migrations/mysql/2026-09-02-120000_add_key_id/down.sql`
- `migrations/mysql/2026-09-02-120000_add_key_id/up.sql`
- `migrations/postgresql/2026-09-02-120000_add_key_id/down.sql`
- `migrations/postgresql/2026-09-02-120000_add_key_id/up.sql`
- `migrations/sqlite/2026-09-02-120000_add_key_id/down.sql`
- `migrations/sqlite/2026-09-02-120000_add_key_id/up.sql`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/events.rs`
- `src/api/core/organizations.rs`
- `src/db/models/event.rs`
- `src/db/models/mod.rs`
- `src/db/models/org_policy.rs`
- `src/db/models/organization.rs`
- `src/db/models/user.rs`
- `src/db/schema.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
