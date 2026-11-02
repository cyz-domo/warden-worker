# Vaultwarden 上游变更摘要（2026-11-01）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`0f44744ef8` → `6ed6921cc2`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/0f44744ef86a2a7bea615af1c024044ce5736187...6ed6921cc2b0e03b4d74189574d2a5e71c512e97)）
- 提交数：15；变更文件数：32
- 生成时间：2026-11-02 09:24 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`e2d63cfc`](https://github.com/dani-garcia/vaultwarden/commit/e2d63cfca31aab36c95de8304cd995d170db962e) Remove expired cookies from the icon cookie jar (#7905)
- [`a96912bf`](https://github.com/dani-garcia/vaultwarden/commit/a96912bf21a9baa3a35e0506d5a110572840d1a3) fix(attachments): enforce quota at v2 metadata reservation (#7450)
- [`64ae55ce`](https://github.com/dani-garcia/vaultwarden/commit/64ae55ceb2e33aa08e7291d50ca010c679064842) Align cipher org checks & attachment notification with upstream (#7906)
- [`b723afe0`](https://github.com/dani-garcia/vaultwarden/commit/b723afe08a57cb83d7fbe77af11e4fabacf3cb22) Document two feature flags in .env.template (#7841)
- [`80d18594`](https://github.com/dani-garcia/vaultwarden/commit/80d1859452bc15bec3d03261dc6c41ea877a75b4) Fix push device unregistration endpoint (#7891)
- [`be5d3ad9`](https://github.com/dani-garcia/vaultwarden/commit/be5d3ad907bbe560dfe77b5078c6e6823267e95d) Don't return internal error details in bulk member responses; gate SSO token logging (#7777)
- [`863395f5`](https://github.com/dani-garcia/vaultwarden/commit/863395f5e28eab5f939e341bb700320c5eb8e0d7) Add configurable Fill Assist rules URL (#7791)
- [`42660af7`](https://github.com/dani-garcia/vaultwarden/commit/42660af757559a1d530a0a1fe42581d013aca1d8) Add registration email verification clicked endpoint (#7790)
- [`0dbc92f2`](https://github.com/dani-garcia/vaultwarden/commit/0dbc92f2b709ab0dc5b58a6633cbad4dda5d4603) Make 2FA remember duration configurable (#7421)
- [`f1c7eccd`](https://github.com/dani-garcia/vaultwarden/commit/f1c7eccd4a3dc9505b87d1f71f915e8bb503466b) Bump user revision once for bulk cipher move (#7840)
- [`e056359a`](https://github.com/dani-garcia/vaultwarden/commit/e056359a42656ecb844c856dcc9cff36ca322224) Send logout notification when deleting a user (#7839)
- [`acb574b7`](https://github.com/dani-garcia/vaultwarden/commit/acb574b7684876aa54ff3e4246316b1eccdf6627) Two small fixes for upstream parity (#7918)
- [`79ee29d5`](https://github.com/dani-garcia/vaultwarden/commit/79ee29d501624b25e063b86bddf1ba57d4e3092b) Add HTTP auth and policy revocation audit events (#7919)
- [`5d870b79`](https://github.com/dani-garcia/vaultwarden/commit/5d870b796fddcaf0fc35691b688933cdada70259) Clippy fixes, crate and web-vault updates (#7920)
- [`6ed6921c`](https://github.com/dani-garcia/vaultwarden/commit/6ed6921cc2b0e03b4d74189574d2a5e71c512e97) Escape request values in log messages (#7921)

## 变更文件（前 200 个）

- modified `.env.template` (+11/-1)
- modified `Cargo.lock` (+486/-148)
- modified `Cargo.toml` (+5/-5)
- modified `docker/DockerSettings.yaml` (+2/-2)
- modified `docker/Dockerfile.alpine` (+6/-6)
- modified `docker/Dockerfile.debian` (+6/-6)
- modified `src/api/admin.rs` (+11/-3)
- modified `src/api/core/accounts.rs` (+35/-9)
- modified `src/api/core/ciphers.rs` (+100/-65)
- modified `src/api/core/events.rs` (+2/-2)
- modified `src/api/core/mod.rs` (+1/-0)
- modified `src/api/core/organizations.rs` (+100/-23)
- modified `src/api/core/sends.rs` (+6/-5)
- modified `src/api/core/two_factor/email.rs` (+13/-5)
- modified `src/api/core/two_factor/mod.rs` (+11/-3)
- modified `src/api/icons.rs` (+12/-1)
- modified `src/api/identity.rs` (+28/-1)
- modified `src/api/mod.rs` (+2/-2)
- modified `src/api/push.rs` (+34/-24)
- modified `src/api/web.rs` (+1/-1)
- modified `src/auth.rs` (+5/-1)
- modified `src/auth/send.rs` (+5/-1)
- modified `src/config.rs` (+26/-1)
- modified `src/db/models/cipher.rs` (+9/-0)
- modified `src/db/models/event.rs` (+3/-2)
- modified `src/db/models/mod.rs` (+1/-1)
- modified `src/db/models/org_policy.rs` (+10/-0)
- modified `src/db/models/send.rs` (+0/-1)
- modified `src/db/models/user.rs` (+2/-2)
- modified `src/mail.rs` (+1/-1)
- modified `src/sso.rs` (+7/-3)
- modified `src/sso_client.rs` (+1/-1)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/core/accounts.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/events.rs`
- `src/api/core/mod.rs`
- `src/api/core/organizations.rs`
- `src/api/core/sends.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/icons.rs`
- `src/api/identity.rs`
- `src/api/mod.rs`
- `src/api/push.rs`
- `src/api/web.rs`
- `src/auth/send.rs`
- `src/db/models/cipher.rs`
- `src/db/models/event.rs`
- `src/db/models/mod.rs`
- `src/db/models/org_policy.rs`
- `src/db/models/send.rs`
- `src/db/models/user.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
