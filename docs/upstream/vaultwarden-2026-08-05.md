# Vaultwarden 上游变更摘要（2026-08-05）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`d6a3d539ed` → `55f883a566`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/d6a3d539ed13352085ca7dfa63c49017d86c419b...55f883a5669a5b1c0227bc8341e7a2899da20660)）
- 提交数：20；变更文件数：75
- 生成时间：2026-08-06 09:19 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`b25f7153`](https://github.com/dani-garcia/vaultwarden/commit/b25f715364946e626c7d5ca299609085ba8e1d47) Fix enforce blocked (#7246)
- [`ec7fa137`](https://github.com/dani-garcia/vaultwarden/commit/ec7fa137b7afd15ab13af6dcecc530661e62cd45) Admin password recovery endpoint change (#7270)
- [`fddc16d2`](https://github.com/dani-garcia/vaultwarden/commit/fddc16d2b87878e938f0dabaede9d728e827fd50) fix(sends): emit hideEmail as non-null boolean in sync response (#7283)
- [`a16b5afa`](https://github.com/dani-garcia/vaultwarden/commit/a16b5afaaa5f9c546566a3d0cc0102f43b3edb95) Org membership delete remove Invitation (#7284)
- [`a058a35c`](https://github.com/dani-garcia/vaultwarden/commit/a058a35ccddf48e77665bcf9b3f5fc2711f63f87) [v2026.5.0] Registration request update (#7295)
- [`7320a1db`](https://github.com/dani-garcia/vaultwarden/commit/7320a1db4b1124d53c2fe316ced8cd3acb2c0a1c) PutPolicy now using vnext format (#7296)
- [`5c5e8e1a`](https://github.com/dani-garcia/vaultwarden/commit/5c5e8e1a6ff8ad1fb8d2b170a71f14f8e96d3d35) 2026.6.0 send support (#7346)
- [`5447ee6a`](https://github.com/dani-garcia/vaultwarden/commit/5447ee6af27b9780e14a7c6ebe7a330820df8f48) SSO use ClientSecretPost if ClientSecretBasic is not available (#7357)
- [`4720cdbe`](https://github.com/dani-garcia/vaultwarden/commit/4720cdbe8660a40b40754046fe3763eb34c735f5) Add `pm-26340-linux-biometrics-v2` feature flag (#7358)
- [`64d28ab6`](https://github.com/dani-garcia/vaultwarden/commit/64d28ab66e10cee86ef62d1c4874c1919daa7e69) improve CI (#6991)
- [`169aa5ef`](https://github.com/dani-garcia/vaultwarden/commit/169aa5efcc8d94684ff3bc813a00e6bcc0cc537a) Misc updates and fixes (#7406)
- [`4a9bcb06`](https://github.com/dani-garcia/vaultwarden/commit/4a9bcb069465e20e487c5e8cad6fdad8b2301a94) Remove old compatibility code (#7434)
- [`683a23e4`](https://github.com/dani-garcia/vaultwarden/commit/683a23e43c5a440cab80300f47cb0d2639e616fa) Fix compilation with newer `rust-musl` version (#7453)
- [`660faee6`](https://github.com/dani-garcia/vaultwarden/commit/660faee68e3406d33244b67eadc18524c47674c2) Fix custom role dialog selectors (#7442)
- [`5040bcb7`](https://github.com/dani-garcia/vaultwarden/commit/5040bcb7c0d23623cd7ed39f3aed6ec2bd5c2377) Remove unused fields (#7458)
- [`a6a88e79`](https://github.com/dani-garcia/vaultwarden/commit/a6a88e7929f5d6c8feff6d924abc79133e34f5f1) Update API response, crates and GHA (#7470)
- [`46ae59ea`](https://github.com/dani-garcia/vaultwarden/commit/46ae59eaf444f0ae0a799070cf2bd6c415284a51) Trusted proxy support, unauthenticated rate limit & other fixes (#7472)
- [`2629bcbe`](https://github.com/dani-garcia/vaultwarden/commit/2629bcbe1380c894e3a7f52cafcac3988edb8fbb) Always send initOrganization and orgUserHasExistingUser in org invite URL (#7482)
- [`74ceaf23`](https://github.com/dani-garcia/vaultwarden/commit/74ceaf23549240bded0a89ae038258bcff6d27a3) Fix Debian cross-linking with xx-cargo (#7524)
- [`55f883a5`](https://github.com/dani-garcia/vaultwarden/commit/55f883a5669a5b1c0227bc8341e7a2899da20660) Fix playwright test (#7548)

## 变更文件（前 200 个）

- modified `.env.template` (+24/-0)
- modified `.github/workflows/build.yml` (+1/-1)
- modified `.github/workflows/check-templates.yml` (+1/-1)
- modified `.github/workflows/hadolint.yml` (+13/-12)
- modified `.github/workflows/release.yml` (+15/-15)
- modified `.github/workflows/trivy.yml` (+2/-2)
- modified `.github/workflows/typos.yml` (+2/-2)
- modified `.github/workflows/zizmor.yml` (+3/-3)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+492/-695)
- modified `Cargo.toml` (+31/-28)
- modified `docker/DockerSettings.yaml` (+4/-4)
- modified `docker/Dockerfile.alpine` (+12/-12)
- modified `docker/Dockerfile.debian` (+19/-9)
- modified `docker/Dockerfile.j2` (+7/-2)
- modified `macros/Cargo.toml` (+2/-2)
- modified `playwright/.env.template` (+13/-3)
- modified `playwright/README.md` (+14/-16)
- modified `playwright/compose/keycloak/setup.sh` (+2/-2)
- modified `playwright/compose/playwright/Dockerfile` (+1/-1)
- modified `playwright/compose/warden/Dockerfile` (+1/-0)
- modified `playwright/compose/warden/build.sh` (+11/-0)
- modified `playwright/docker-compose.yml` (+9/-6)
- modified `playwright/global-setup.ts` (+1/-1)
- modified `playwright/global-utils.ts` (+2/-13)
- modified `playwright/package-lock.json` (+582/-580)
- modified `playwright/package.json` (+7/-7)
- modified `playwright/playwright.config.ts` (+10/-4)
- modified `playwright/test.env` (+6/-4)
- modified `playwright/tests/collection.spec.ts` (+5/-11)
- added `playwright/tests/cyphers.spec.ts` (+56/-0)
- modified `playwright/tests/login.smtp.spec.ts` (+6/-25)
- modified `playwright/tests/login.spec.ts` (+2/-2)
- modified `playwright/tests/organization.smtp.spec.ts` (+49/-13)
- added `playwright/tests/secrets.spec.ts` (+110/-0)
- added `playwright/tests/send.spec.ts` (+72/-0)
- modified `playwright/tests/setups/2fa.ts` (+8/-7)
- added `playwright/tests/setups/admin.ts` (+21/-0)
- modified `playwright/tests/setups/db-teardown.ts` (+1/-1)
- modified `playwright/tests/setups/orgs.ts` (+14/-11)
- modified `playwright/tests/setups/sso-teardown.ts` (+1/-1)
- modified `playwright/tests/setups/sso.ts` (+8/-17)
- modified `playwright/tests/setups/user.ts` (+25/-9)
- modified `playwright/tests/sso_login.smtp.spec.ts` (+49/-6)
- modified `playwright/tests/sso_login.spec.ts` (+6/-4)
- modified `playwright/tests/sso_organization.smtp.spec.ts` (+7/-9)
- modified `playwright/tests/sso_organization.spec.ts` (+19/-8)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/core/accounts.rs` (+122/-23)
- modified `src/api/core/ciphers.rs` (+14/-5)
- modified `src/api/core/events.rs` (+10/-4)
- modified `src/api/core/mod.rs` (+9/-2)
- modified `src/api/core/organizations.rs` (+119/-34)
- modified `src/api/core/public.rs` (+9/-2)
- modified `src/api/core/sends.rs` (+67/-31)
- modified `src/api/icons.rs` (+17/-1)
- modified `src/api/identity.rs` (+23/-2)
- modified `src/api/notifications.rs` (+60/-10)
- modified `src/auth.rs` (+57/-16)
- added `src/auth/send.rs` (+147/-0)
- modified `src/config.rs` (+29/-0)
- modified `src/db/models/cipher.rs` (+16/-20)
- modified `src/db/models/collection.rs` (+47/-0)
- modified `src/db/models/group.rs` (+3/-1)
- modified `src/db/models/mod.rs` (+1/-4)
- modified `src/db/models/organization.rs` (+15/-2)
- modified `src/db/models/send.rs` (+83/-41)
- modified `src/db/models/user.rs` (+17/-0)
- modified `src/error.rs` (+19/-5)
- modified `src/http_client.rs` (+56/-23)
- modified `src/mail.rs` (+12/-3)
- modified `src/main.rs` (+1/-1)
- modified `src/ratelimit.rs` (+18/-0)
- modified `src/sso_client.rs` (+34/-13)
- modified `src/static/templates/scss/vaultwarden.scss.hbs` (+2/-2)

## 可能影响协议/客户端兼容性的文件

- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/events.rs`
- `src/api/core/mod.rs`
- `src/api/core/organizations.rs`
- `src/api/core/public.rs`
- `src/api/core/sends.rs`
- `src/api/icons.rs`
- `src/api/identity.rs`
- `src/api/notifications.rs`
- `src/auth/send.rs`
- `src/db/models/cipher.rs`
- `src/db/models/collection.rs`
- `src/db/models/group.rs`
- `src/db/models/mod.rs`
- `src/db/models/organization.rs`
- `src/db/models/send.rs`
- `src/db/models/user.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
