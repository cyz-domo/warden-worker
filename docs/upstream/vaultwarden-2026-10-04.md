# Vaultwarden 上游变更摘要（2026-10-04）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`061694d0cb` → `1f99d9d276`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/061694d0cb3bbf5d4c7e920c892824f0020cff83...1f99d9d276c41107eeaddd086ef840c28279cb7d)）
- 提交数：7；变更文件数：26
- 生成时间：2026-10-05 08:57 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`44444420`](https://github.com/dani-garcia/vaultwarden/commit/4444442075b02d55abef7a071da642312d4abf1e) Add `undetermined-cipher-scenario-logic` feature flag (#7802)
- [`cb89088b`](https://github.com/dani-garcia/vaultwarden/commit/cb89088bc311c5b5615c11c018122a4dfcf26557) Add Windows native credential sync to supported feature flags (#7798)
- [`f9f011d5`](https://github.com/dani-garcia/vaultwarden/commit/f9f011d50cc840d02d0b8a59f87511e943381948) Hide the whole change-email section when EMAIL_CHANGE_ALLOWED is false (#7759)
- [`3714b504`](https://github.com/dani-garcia/vaultwarden/commit/3714b504f6eb4f222b621ece7e1ba9fdfc813f05) Fix Clippy warnings across all targets (#7782)
- [`1c89e177`](https://github.com/dani-garcia/vaultwarden/commit/1c89e1772bfe51edf362fd033cf9c492ccd7ee55) 2fa email fallback need a verified email (#7770)
- [`f4f1a8e1`](https://github.com/dani-garcia/vaultwarden/commit/f4f1a8e105ec5fd72ec1dd1bed800bcf44deae2b) Send API cleanup: remove legacy endpoints and align access with upstream (#7806)
- [`1f99d9d2`](https://github.com/dani-garcia/vaultwarden/commit/1f99d9d276c41107eeaddd086ef840c28279cb7d) Remove legacy API endpoints and compatibility code dropped upstream (#7809)

## 变更文件（前 200 个）

- modified `.env.template` (+2/-19)
- modified `src/api/admin.rs` (+14/-13)
- modified `src/api/core/accounts.rs` (+127/-71)
- modified `src/api/core/ciphers.rs` (+1/-1)
- modified `src/api/core/organizations.rs` (+16/-223)
- modified `src/api/core/sends.rs` (+18/-178)
- modified `src/api/core/two_factor/authenticator.rs` (+2/-1)
- modified `src/api/core/two_factor/duo.rs` (+3/-116)
- modified `src/api/core/two_factor/duo_oidc.rs` (+2/-2)
- modified `src/api/core/two_factor/email.rs` (+7/-6)
- modified `src/api/core/two_factor/mod.rs` (+2/-13)
- modified `src/api/core/two_factor/webauthn.rs` (+2/-1)
- modified `src/api/identity.rs` (+52/-112)
- modified `src/api/web.rs` (+1/-0)
- modified `src/auth.rs` (+1/-1)
- modified `src/auth/send.rs` (+1/-5)
- modified `src/config.rs` (+3/-45)
- modified `src/db/models/cipher.rs` (+1/-0)
- modified `src/db/models/group.rs` (+0/-21)
- modified `src/db/models/organization.rs` (+3/-5)
- modified `src/db/models/send.rs` (+7/-23)
- modified `src/db/models/user.rs` (+5/-1)
- modified `src/http_client.rs` (+9/-9)
- modified `src/main.rs` (+1/-1)
- modified `src/static/templates/scss/vaultwarden.scss.hbs` (+7/-25)
- modified `src/util.rs` (+7/-7)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/organizations.rs`
- `src/api/core/sends.rs`
- `src/api/core/two_factor/authenticator.rs`
- `src/api/core/two_factor/duo.rs`
- `src/api/core/two_factor/duo_oidc.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/core/two_factor/webauthn.rs`
- `src/api/identity.rs`
- `src/api/web.rs`
- `src/auth/send.rs`
- `src/db/models/cipher.rs`
- `src/db/models/group.rs`
- `src/db/models/organization.rs`
- `src/db/models/send.rs`
- `src/db/models/user.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
