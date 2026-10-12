# Vaultwarden 上游变更摘要（2026-10-10）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`1f99d9d276` → `64895524e4`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/1f99d9d276c41107eeaddd086ef840c28279cb7d...64895524e481492130061b95aa4433118f6b1c59)）
- 提交数：16；变更文件数：58
- 生成时间：2026-10-12 09:07 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`895ec2b1`](https://github.com/dani-garcia/vaultwarden/commit/895ec2b1e7ac0f78d2c8ee5f88fe048c20fdfb19) Align API behaviour with upstream and return errors on malformed input (#7810)
- [`7c188967`](https://github.com/dani-garcia/vaultwarden/commit/7c188967e7a467b19b2757bc3326f9cf3ccae327) Update crates, Rust and other dependencies (#7814)
- [`42aa3ee1`](https://github.com/dani-garcia/vaultwarden/commit/42aa3ee1c77758086abe8cb16df7d9f222fe6885) Close WebSocket connections that stop responding (#7807)
- [`415df400`](https://github.com/dani-garcia/vaultwarden/commit/415df400f49d35a2999fa0292c8d244664f58b26) Validate the Duo and SSO purge schedules on startup (#7819)
- [`de6d2066`](https://github.com/dani-garcia/vaultwarden/commit/de6d2066b32c913b27c63abc84b23542b2e5517e) docs: fix "runnning" typo in playwright README (#7787)
- [`8b569260`](https://github.com/dani-garcia/vaultwarden/commit/8b56926077aa3976403451d17b897ae00ce3957c) docs: fix Playwright test env link (#7739)
- [`c687cacb`](https://github.com/dani-garcia/vaultwarden/commit/c687cacb6b96f4d4ca24f22c5ce12417373f1f5d) Fix group import to update groups on data change (#7671)
- [`8647af8f`](https://github.com/dani-garcia/vaultwarden/commit/8647af8f89a7d639efd79c339dabbc9215cf8dde) [Web 2026.9.0] Support new Emergency Access password payload (#7746)
- [`b8089e31`](https://github.com/dani-garcia/vaultwarden/commit/b8089e31c2f35958021a32414411aecaddf3b24b) [Web 2026.9.0] Support new account recovery password payload (#7747)
- [`acbf4901`](https://github.com/dani-garcia/vaultwarden/commit/acbf49018f616e52e9310789cf861f6e3923c8be) [web-v2026.7.1] 2FA userVerificationToken for Authenticator, Email and Duo (#7563)
- [`a2efadc6`](https://github.com/dani-garcia/vaultwarden/commit/a2efadc650676e98cc1a6152141b6f930f496109) Fix email 2FA for SSO logins on iOS and Android (#7827)
- [`1f802f8e`](https://github.com/dani-garcia/vaultwarden/commit/1f802f8e6ae9b788e1e9cc5dffb7e83780417332) Update web-vault to v2026.8.0 (#7828)
- [`d1cbd027`](https://github.com/dani-garcia/vaultwarden/commit/d1cbd027cdb601cf32bd95ce3c8487ded54dd7fd) Add revision_date to org policies (#7571)
- [`0e93d15b`](https://github.com/dani-garcia/vaultwarden/commit/0e93d15b73fb31b91615a086784fdaec9d0493b2) Add Windows desktop autotype support (#7808)
- [`df2cd3c8`](https://github.com/dani-garcia/vaultwarden/commit/df2cd3c8693782eeedfd1f70318f6f14033213a8) Add `biometrics-sdk-ipc` feature flag (#7813)
- [`64895524`](https://github.com/dani-garcia/vaultwarden/commit/64895524e481492130061b95aa4433118f6b1c59) Delete auth requests when deleting a user (#7837)

## 变更文件（前 200 个）

- modified `.env.template` (+6/-0)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+165/-195)
- modified `Cargo.toml` (+11/-11)
- modified `docker/DockerSettings.yaml` (+2/-2)
- modified `docker/Dockerfile.alpine` (+10/-10)
- modified `docker/Dockerfile.debian` (+7/-7)
- added `migrations/mysql/2026-10-07-120000_add_org_policy_revision_date/down.sql` (+1/-0)
- added `migrations/mysql/2026-10-07-120000_add_org_policy_revision_date/up.sql` (+7/-0)
- added `migrations/postgresql/2026-10-07-120000_add_org_policy_revision_date/down.sql` (+1/-0)
- added `migrations/postgresql/2026-10-07-120000_add_org_policy_revision_date/up.sql` (+9/-0)
- added `migrations/sqlite/2026-10-07-120000_add_org_policy_revision_date/down.sql` (+0/-0)
- added `migrations/sqlite/2026-10-07-120000_add_org_policy_revision_date/up.sql` (+6/-0)
- modified `playwright/README.md` (+2/-2)
- added `playwright/tests/emergency.spec.ts` (+131/-0)
- modified `playwright/tests/login.spec.ts` (+29/-8)
- modified `playwright/tests/setups/2fa.ts` (+18/-1)
- modified `playwright/tests/setups/user.ts` (+16/-2)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/core/accounts.rs` (+76/-22)
- modified `src/api/core/ciphers.rs` (+14/-7)
- modified `src/api/core/emergency_access.rs` (+29/-7)
- modified `src/api/core/events.rs` (+22/-25)
- modified `src/api/core/organizations.rs` (+39/-12)
- modified `src/api/core/public.rs` (+9/-23)
- modified `src/api/core/sends.rs` (+4/-4)
- modified `src/api/core/two_factor/authenticator.rs` (+23/-45)
- modified `src/api/core/two_factor/duo.rs` (+48/-77)
- modified `src/api/core/two_factor/email.rs` (+71/-52)
- modified `src/api/core/two_factor/mod.rs` (+15/-44)
- modified `src/api/core/two_factor/protected_actions.rs` (+2/-3)
- modified `src/api/core/two_factor/webauthn.rs` (+111/-72)
- modified `src/api/core/two_factor/yubikey.rs` (+56/-46)
- modified `src/api/icons.rs` (+2/-2)
- modified `src/api/identity.rs` (+54/-37)
- modified `src/api/mod.rs` (+19/-1)
- modified `src/api/notifications.rs` (+33/-5)
- modified `src/auth.rs` (+95/-11)
- added `src/auth/two_factor.rs` (+281/-0)
- modified `src/config.rs` (+14/-1)
- modified `src/db/models/attachment.rs` (+1/-1)
- modified `src/db/models/auth_request.rs` (+25/-1)
- modified `src/db/models/org_policy.rs` (+13/-7)
- modified `src/db/models/organization.rs` (+1/-1)
- modified `src/db/models/two_factor.rs` (+2/-2)
- modified `src/db/models/user.rs` (+29/-18)
- modified `src/db/schema.rs` (+1/-0)
- modified `src/mail.rs` (+15/-0)
- modified `src/sso.rs` (+17/-15)
- modified `src/sso_client.rs` (+12/-7)
- modified `src/static/scripts/datatables.css` (+2/-2)
- modified `src/static/scripts/datatables.js` (+396/-254)
- added `src/static/templates/email/recover_twofactor.hbs` (+9/-0)
- added `src/static/templates/email/recover_twofactor.html.hbs` (+26/-0)
- modified `src/static/templates/email/send_emergency_access_invite.html.hbs` (+1/-1)
- modified `src/util.rs` (+3/-3)

## 可能影响协议/客户端兼容性的文件

- `migrations/mysql/2026-10-07-120000_add_org_policy_revision_date/down.sql`
- `migrations/mysql/2026-10-07-120000_add_org_policy_revision_date/up.sql`
- `migrations/postgresql/2026-10-07-120000_add_org_policy_revision_date/down.sql`
- `migrations/postgresql/2026-10-07-120000_add_org_policy_revision_date/up.sql`
- `migrations/sqlite/2026-10-07-120000_add_org_policy_revision_date/down.sql`
- `migrations/sqlite/2026-10-07-120000_add_org_policy_revision_date/up.sql`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/emergency_access.rs`
- `src/api/core/events.rs`
- `src/api/core/organizations.rs`
- `src/api/core/public.rs`
- `src/api/core/sends.rs`
- `src/api/core/two_factor/authenticator.rs`
- `src/api/core/two_factor/duo.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/core/two_factor/protected_actions.rs`
- `src/api/core/two_factor/webauthn.rs`
- `src/api/core/two_factor/yubikey.rs`
- `src/api/icons.rs`
- `src/api/identity.rs`
- `src/api/mod.rs`
- `src/api/notifications.rs`
- `src/auth/two_factor.rs`
- `src/db/models/attachment.rs`
- `src/db/models/auth_request.rs`
- `src/db/models/org_policy.rs`
- `src/db/models/organization.rs`
- `src/db/models/two_factor.rs`
- `src/db/models/user.rs`
- `src/db/schema.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
