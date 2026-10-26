# Vaultwarden 上游变更摘要（2026-10-25）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`64895524e4` → `0f44744ef8`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/64895524e481492130061b95aa4433118f6b1c59...0f44744ef86a2a7bea615af1c024044ce5736187)）
- 提交数：10；变更文件数：49
- 生成时间：2026-10-26 07:44 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`58a980f8`](https://github.com/dani-garcia/vaultwarden/commit/58a980f850464f7d9cdc99a765df36f0a3308b91) Add unsupported riscv64 architecture (#7859)
- [`970cc491`](https://github.com/dani-garcia/vaultwarden/commit/970cc491821c83485bf81f655d199950178f0207) api/icons: fix typos in the minimum PNG size comment (#7867)
- [`29659900`](https://github.com/dani-garcia/vaultwarden/commit/296599003c8c08f3afff530d6f265fb86446fb4a) docs: fix connect-src description grammar in config.rs and .env.template (#7866)
- [`f4696877`](https://github.com/dani-garcia/vaultwarden/commit/f4696877b91d4eb6cbeb2c6a4c86a3d19033ccb4) Update dependencies (#7882)
- [`b71cb7a8`](https://github.com/dani-garcia/vaultwarden/commit/b71cb7a832080d9fdfdd97bcccf065307cf73236) Misc fixes regarding jobs and config (#7858)
- [`c925c04d`](https://github.com/dani-garcia/vaultwarden/commit/c925c04de088104f365f8fe9ce6a81da6bfcf72f) Send null org key for unconfirmed members in sync (#7850)
- [`7ce705be`](https://github.com/dani-garcia/vaultwarden/commit/7ce705be56f7df3391f9c8e2359e2fe512ca2fae) Return stored manage flag for groups in collections/details (#7795)
- [`e5a90968`](https://github.com/dani-garcia/vaultwarden/commit/e5a90968f29d911c713d509b0a0a490391865404) Fix bulk cipher share response for SDK clients (#7831)
- [`44c4853d`](https://github.com/dani-garcia/vaultwarden/commit/44c4853d373c35774709a4f3129133f5d37ca204) Upgrade to Rocket v0.5.2 (#7890)
- [`0f44744e`](https://github.com/dani-garcia/vaultwarden/commit/0f44744ef86a2a7bea615af1c024044ce5736187) Fix typo (#7864)

## 变更文件（前 200 个）

- modified `.env.template` (+5/-5)
- modified `.github/workflows/build.yml` (+1/-1)
- modified `.github/workflows/release.yml` (+10/-10)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.github/workflows/zizmor.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+341/-388)
- modified `Cargo.toml` (+13/-13)
- modified `docker/DockerSettings.yaml` (+2/-2)
- modified `docker/Dockerfile.alpine` (+4/-4)
- modified `docker/Dockerfile.debian` (+2/-2)
- modified `docker/README.md` (+1/-0)
- modified `docker/docker-bake.hcl` (+6/-0)
- modified `macros/Cargo.toml` (+1/-1)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/admin.rs` (+9/-2)
- modified `src/api/core/ciphers.rs` (+60/-6)
- modified `src/api/core/emergency_access.rs` (+178/-106)
- modified `src/api/core/events.rs` (+8/-5)
- modified `src/api/core/two_factor/email.rs` (+2/-2)
- modified `src/api/core/two_factor/mod.rs` (+14/-4)
- modified `src/api/core/two_factor/protected_actions.rs` (+5/-4)
- modified `src/api/icons.rs` (+4/-4)
- modified `src/api/identity.rs` (+2/-2)
- modified `src/api/mod.rs` (+1/-1)
- modified `src/api/notifications.rs` (+3/-6)
- modified `src/api/web.rs` (+62/-32)
- modified `src/auth.rs` (+42/-24)
- modified `src/config.rs` (+110/-36)
- modified `src/db/mod.rs` (+3/-3)
- modified `src/db/models/auth_request.rs` (+1/-1)
- modified `src/db/models/cipher.rs` (+16/-6)
- modified `src/db/models/emergency_access.rs` (+50/-17)
- modified `src/db/models/event.rs` (+17/-11)
- modified `src/db/models/group.rs` (+5/-5)
- modified `src/db/models/mod.rs` (+3/-1)
- modified `src/db/models/organization.rs` (+2/-1)
- modified `src/db/models/two_factor_incomplete.rs` (+2/-2)
- modified `src/error.rs` (+2/-2)
- modified `src/mail.rs` (+8/-8)
- modified `src/main.rs` (+5/-3)
- modified `src/ratelimit.rs` (+6/-6)
- modified `src/sso_client.rs` (+1/-1)
- modified `src/static/scripts/admin_diagnostics.js` (+9/-2)
- modified `src/static/scripts/datatables.css` (+2/-2)
- modified `src/static/scripts/datatables.js` (+13/-7)
- modified `src/static/templates/admin/diagnostics.hbs` (+17/-3)
- modified `src/util.rs` (+78/-33)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/emergency_access.rs`
- `src/api/core/events.rs`
- `src/api/core/two_factor/email.rs`
- `src/api/core/two_factor/mod.rs`
- `src/api/core/two_factor/protected_actions.rs`
- `src/api/icons.rs`
- `src/api/identity.rs`
- `src/api/mod.rs`
- `src/api/notifications.rs`
- `src/api/web.rs`
- `src/db/mod.rs`
- `src/db/models/auth_request.rs`
- `src/db/models/cipher.rs`
- `src/db/models/emergency_access.rs`
- `src/db/models/event.rs`
- `src/db/models/group.rs`
- `src/db/models/mod.rs`
- `src/db/models/organization.rs`
- `src/db/models/two_factor_incomplete.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
