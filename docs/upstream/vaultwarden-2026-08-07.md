# Vaultwarden 上游变更摘要（2026-08-07）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`55f883a566` → `0cefa4cca7`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/55f883a5669a5b1c0227bc8341e7a2899da20660...0cefa4cca7c9f2a5579dd290f78193b543818c51)）
- 提交数：2；变更文件数：29
- 生成时间：2026-08-10 04:05 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`b30cc085`](https://github.com/dani-garcia/vaultwarden/commit/b30cc08562cf59645271e0431284a88fdab6e27a) Misc fixes and updates (#7558)
- [`0cefa4cc`](https://github.com/dani-garcia/vaultwarden/commit/0cefa4cca7c9f2a5579dd290f78193b543818c51) Include user email in successful login logs (#7496)

## 变更文件（前 200 个）

- modified `.github/workflows/hadolint.yml` (+2/-2)
- modified `.github/workflows/release.yml` (+10/-10)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.github/workflows/zizmor.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+4/-5)
- modified `Cargo.lock` (+165/-173)
- modified `Cargo.toml` (+12/-12)
- modified `docker/DockerSettings.yaml` (+2/-2)
- modified `docker/Dockerfile.alpine` (+11/-10)
- modified `docker/Dockerfile.debian` (+12/-10)
- modified `docker/Dockerfile.j2` (+6/-4)
- modified `src/api/admin.rs` (+31/-0)
- modified `src/api/core/ciphers.rs` (+3/-3)
- modified `src/api/core/two_factor/yubikey.rs` (+44/-13)
- modified `src/api/identity.rs` (+2/-2)
- modified `src/api/mod.rs` (+1/-1)
- modified `src/api/web.rs` (+38/-5)
- modified `src/config.rs` (+6/-0)
- modified `src/db/models/event.rs` (+7/-3)
- modified `src/error.rs` (+1/-1)
- modified `src/static/scripts/admin.js` (+1/-2)
- modified `src/static/scripts/admin_diagnostics.js` (+21/-15)
- modified `src/static/scripts/admin_organizations.js` (+1/-2)
- modified `src/static/scripts/admin_settings.js` (+0/-1)
- modified `src/static/scripts/admin_users.js` (+1/-2)
- modified `src/static/templates/admin/diagnostics.hbs` (+10/-0)
- modified `src/storage.rs` (+2/-2)
- modified `src/util.rs` (+38/-0)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/core/ciphers.rs`
- `src/api/core/two_factor/yubikey.rs`
- `src/api/identity.rs`
- `src/api/mod.rs`
- `src/api/web.rs`
- `src/db/models/event.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
