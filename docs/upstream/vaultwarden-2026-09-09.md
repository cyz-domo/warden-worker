# Vaultwarden 上游变更摘要（2026-09-09）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`a6c3bd6d18` → `eb212e23fa`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/a6c3bd6d1826fb527822df4a2655f34fd440d7d4...eb212e23fad88e6136723f43e5b73543fa7026d3)）
- 提交数：12；变更文件数：41
- 生成时间：2026-09-14 07:51 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`32d85d03`](https://github.com/dani-garcia/vaultwarden/commit/32d85d03bb5ec401d1378f4cd60139be1d8db3f4) Fix organization import failing with missing field groups (#7699)
- [`2ffad877`](https://github.com/dani-garcia/vaultwarden/commit/2ffad8775d8712329aab7d00a05a94f64098170b) Add `pm-32413-multi-client-password-management` feature flag (#7677)
- [`277e1536`](https://github.com/dani-garcia/vaultwarden/commit/277e1536ebe426296519f8bd8bf99f5f6c1c5c77) Log IP/username on two-factor email-login credential failures (#7654)
- [`57fbed1b`](https://github.com/dani-garcia/vaultwarden/commit/57fbed1bed2e42b540cb790dd536e02633c4f445) Support admin reset 2fa (#7435)
- [`f1ff6130`](https://github.com/dani-garcia/vaultwarden/commit/f1ff61300844b0664907393c0fe93f5092654784) fix(security): revoke 2FA remember tokens when credentials or 2FA change (#7682)
- [`f1c36b8c`](https://github.com/dani-garcia/vaultwarden/commit/f1c36b8c1d9b2cdd0f1cf6f1c4062f81c3a70302) fix(security): rate limit prelogin and auth request endpoints (#7681)
- [`b7667e27`](https://github.com/dani-garcia/vaultwarden/commit/b7667e27bf3500a2446d39446b1a7b10b8b25991) fix: Correct invalid comment syntax in .dockerignore (#7274)
- [`de7abaaa`](https://github.com/dani-garcia/vaultwarden/commit/de7abaaafa5ce6627e43efa52840f6df6f43da23) Update Rust and adjust DockerSettings (#7690)
- [`5b51b60f`](https://github.com/dani-garcia/vaultwarden/commit/5b51b60f9407bc4e088eb1dcb035a5d395178aab) Route service clients through shared HTTP setup (#7639)
- [`e992cbb4`](https://github.com/dani-garcia/vaultwarden/commit/e992cbb4f520c53d50b22fa3cc1f2f77c4a95a81) Fix iOS registration token response (#7714)
- [`25dfedaf`](https://github.com/dani-garcia/vaultwarden/commit/25dfedafd73cdeee47a50c1dfa7a451afe1ae1bd) Use insert_into when possible (#6437)
- [`eb212e23`](https://github.com/dani-garcia/vaultwarden/commit/eb212e23fad88e6136723f43e5b73543fa7026d3) Fix archiveDate update (#7722)

## 变更文件（前 200 个）

- modified `.dockerignore` (+2/-2)
- modified `.env.template` (+1/-0)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.github/workflows/zizmor.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+213/-159)
- modified `Cargo.toml` (+11/-10)
- modified `docker/DockerSettings.yaml` (+2/-1)
- modified `docker/Dockerfile.alpine` (+4/-4)
- modified `docker/Dockerfile.debian` (+1/-1)
- modified `docker/render_template` (+8/-2)
- modified `macros/Cargo.toml` (+1/-1)
- modified `playwright/tests/organization.smtp.spec.ts` (+6/-1)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/core/accounts.rs` (+12/-6)
- modified `src/api/core/ciphers.rs` (+4/-3)
- modified `src/api/core/organizations.rs` (+68/-35)
- modified `src/api/core/two_factor/email.rs` (+12/-3)
- modified `src/api/core/two_factor/mod.rs` (+3/-2)
- modified `src/api/identity.rs` (+23/-6)
- modified `src/auth.rs` (+9/-3)
- modified `src/config.rs` (+2/-1)
- modified `src/db/models/archive.rs` (+6/-3)
- modified `src/db/models/attachment.rs` (+7/-15)
- modified `src/db/models/auth_request.rs` (+11/-19)
- modified `src/db/models/cipher.rs` (+7/-15)
- modified `src/db/models/collection.rs` (+30/-62)
- modified `src/db/models/device.rs` (+18/-3)
- modified `src/db/models/emergency_access.rs` (+7/-15)
- modified `src/db/models/event.rs` (+22/-13)
- modified `src/db/models/folder.rs` (+12/-21)
- modified `src/db/models/group.rs` (+39/-88)
- modified `src/db/models/organization.rs` (+21/-47)
- modified `src/db/models/send.rs` (+7/-15)
- modified `src/db/models/user.rs` (+10/-8)
- modified `src/http_client.rs` (+73/-14)
- modified `src/mail.rs` (+12/-2)
- added `src/static/templates/email/admin_account_recovery.hbs` (+12/-0)
- renamed `src/static/templates/email/admin_account_recovery.html.hbs` (+9/-2)
- removed `src/static/templates/email/admin_reset_password.hbs` (+0/-4)
- modified `src/storage.rs` (+12/-7)

## 可能影响协议/客户端兼容性的文件

- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/organizations.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/identity.rs`
- `src/db/models/archive.rs`
- `src/db/models/attachment.rs`
- `src/db/models/auth_request.rs`
- `src/db/models/cipher.rs`
- `src/db/models/collection.rs`
- `src/db/models/device.rs`
- `src/db/models/emergency_access.rs`
- `src/db/models/event.rs`
- `src/db/models/folder.rs`
- `src/db/models/group.rs`
- `src/db/models/organization.rs`
- `src/db/models/send.rs`
- `src/db/models/user.rs`
- `src/static/templates/email/admin_account_recovery.hbs`
- `src/static/templates/email/admin_account_recovery.html.hbs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
