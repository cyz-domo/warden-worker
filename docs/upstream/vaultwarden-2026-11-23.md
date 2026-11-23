# Vaultwarden 上游变更摘要（2026-11-23）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`4051a4d3a6` → `6f7a7f7034`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/4051a4d3a64d8d9f6e019c8c1646fbed54384d15...6f7a7f70347e0cdde9d3506342ff04b22a9c78ce)）
- 提交数：8；变更文件数：68
- 生成时间：2026-11-23 10:41 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`4a0acc33`](https://github.com/dani-garcia/vaultwarden/commit/4a0acc3314ee74cfa05d4fb803a9781d9b3caff0) Modernize Rust code and Fixes (#7971)
- [`baff1cab`](https://github.com/dani-garcia/vaultwarden/commit/baff1cabae39f55fd5111d0724d1389d08d00a63) Misc updates (#7980)
- [`2c9b7c06`](https://github.com/dani-garcia/vaultwarden/commit/2c9b7c06a9cbeda00e74949624740b6f0b852088) Return an error instead of panicking on invalid attachment/cipher input (#7989)
- [`0a4909cf`](https://github.com/dani-garcia/vaultwarden/commit/0a4909cfe03d2020af39b2ea508fc1afb46d8c1a) Start support for sync (un)lock in 2026.11.0 onwards (#7986)
- [`c6e7da4b`](https://github.com/dani-garcia/vaultwarden/commit/c6e7da4b5c36495568923694031d5406deb3c736) Add the Send controls policy (policy 21) (#7519)
- [`010c269e`](https://github.com/dani-garcia/vaultwarden/commit/010c269e3751b0a9da1b21a4a7a2aadd1ce6d02a) Improve and expand kdf_upgrade (#7855)
- [`cdf20354`](https://github.com/dani-garcia/vaultwarden/commit/cdf2035433eb355b707b9f1c764636e69cc87a2c) Do not fail login when push device registration fails (#7378)
- [`6f7a7f70`](https://github.com/dani-garcia/vaultwarden/commit/6f7a7f70347e0cdde9d3506342ff04b22a9c78ce) Improve Playwright tests (#7752)

## 变更文件（前 200 个）

- modified `.env.template` (+3/-0)
- modified `.github/workflows/release.yml` (+6/-6)
- modified `.github/workflows/sdk-live-tests.yml` (+4/-1)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.github/workflows/zizmor.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+3/-3)
- modified `.typos.toml` (+3/-2)
- modified `Cargo.lock` (+394/-635)
- modified `Cargo.toml` (+37/-28)
- modified `build.rs` (+0/-7)
- modified `docker/Dockerfile.alpine` (+4/-4)
- modified `docker/Dockerfile.debian` (+1/-1)
- modified `macros/Cargo.toml` (+1/-1)
- modified `macros/src/lib.rs` (+1/-1)
- modified `playwright/compose/warden/build.sh` (+1/-1)
- modified `playwright/docker-compose.yml` (+4/-0)
- modified `playwright/test.env` (+2/-0)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/admin.rs` (+54/-46)
- modified `src/api/core/accounts.rs` (+26/-46)
- modified `src/api/core/ciphers.rs` (+26/-24)
- modified `src/api/core/emergency_access.rs` (+2/-3)
- modified `src/api/core/events.rs` (+4/-6)
- modified `src/api/core/organizations.rs` (+44/-50)
- modified `src/api/core/public.rs` (+2/-2)
- modified `src/api/core/sends.rs` (+17/-60)
- modified `src/api/core/two_factor/authenticator.rs` (+5/-6)
- modified `src/api/core/two_factor/duo.rs` (+4/-4)
- modified `src/api/core/two_factor/duo_oidc.rs` (+15/-24)
- modified `src/api/core/two_factor/email.rs` (+47/-31)
- modified `src/api/core/two_factor/mod.rs` (+3/-2)
- modified `src/api/core/two_factor/webauthn.rs` (+6/-10)
- modified `src/api/core/two_factor/yubikey.rs` (+3/-2)
- modified `src/api/icons.rs` (+162/-196)
- modified `src/api/identity.rs` (+26/-26)
- modified `src/api/mod.rs` (+4/-2)
- modified `src/api/notifications.rs` (+24/-25)
- modified `src/auth.rs` (+32/-31)
- modified `src/auth/send.rs` (+4/-4)
- modified `src/config.rs` (+76/-34)
- modified `src/db/mod.rs` (+17/-11)
- modified `src/db/models/attachment.rs` (+1/-1)
- modified `src/db/models/cipher.rs` (+19/-27)
- modified `src/db/models/collection.rs` (+3/-13)
- modified `src/db/models/device.rs` (+31/-31)
- modified `src/db/models/emergency_access.rs` (+3/-3)
- modified `src/db/models/event.rs` (+2/-6)
- modified `src/db/models/group.rs` (+5/-8)
- modified `src/db/models/org_policy.rs` (+10/-54)
- added `src/db/models/org_policy/send_controls.rs` (+289/-0)
- modified `src/db/models/organization.rs` (+153/-72)
- modified `src/db/models/send.rs` (+8/-8)
- modified `src/db/models/sso_auth.rs` (+1/-1)
- modified `src/db/models/two_factor.rs` (+3/-3)
- modified `src/db/models/user.rs` (+44/-10)
- modified `src/error.rs` (+1/-2)
- modified `src/http_client.rs` (+14/-16)
- modified `src/mail.rs` (+7/-16)
- modified `src/main.rs` (+12/-22)
- modified `src/sso.rs` (+6/-6)
- modified `src/sso_client.rs` (+17/-15)
- modified `src/static/global_domains.json` (+0/-664)
- removed `src/static/images/fallback-icon.png` (+0/-0)
- modified `src/static/scripts/datatables.css` (+2/-2)
- modified `src/static/scripts/datatables.js` (+43/-39)
- modified `src/storage.rs` (+1/-2)
- modified `src/util.rs` (+64/-108)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/emergency_access.rs`
- `src/api/core/events.rs`
- `src/api/core/organizations.rs`
- `src/api/core/public.rs`
- `src/api/core/sends.rs`
- `src/api/core/two_factor/authenticator.rs`
- `src/api/core/two_factor/duo.rs`
- `src/api/core/two_factor/duo_oidc.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/core/two_factor/webauthn.rs`
- `src/api/core/two_factor/yubikey.rs`
- `src/api/icons.rs`
- `src/api/identity.rs`
- `src/api/mod.rs`
- `src/api/notifications.rs`
- `src/auth/send.rs`
- `src/db/mod.rs`
- `src/db/models/attachment.rs`
- `src/db/models/cipher.rs`
- `src/db/models/collection.rs`
- `src/db/models/device.rs`
- `src/db/models/emergency_access.rs`
- `src/db/models/event.rs`
- `src/db/models/group.rs`
- `src/db/models/org_policy.rs`
- `src/db/models/org_policy/send_controls.rs`
- `src/db/models/organization.rs`
- `src/db/models/send.rs`
- `src/db/models/sso_auth.rs`
- `src/db/models/two_factor.rs`
- `src/db/models/user.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
