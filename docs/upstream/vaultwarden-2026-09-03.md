# Vaultwarden 上游变更摘要（2026-09-03）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`fdc156b247` → `a6c3bd6d18`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/fdc156b247846ca73f6aa3e9c676a6f2f57577cf...a6c3bd6d1826fb527822df4a2655f34fd440d7d4)）
- 提交数：2；变更文件数：26
- 生成时间：2026-09-07 07:17 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`6729e835`](https://github.com/dani-garcia/vaultwarden/commit/6729e835218edb29b644a99f65a3b76cde9a341d) Misc Updates (#7676)
- [`a6c3bd6d`](https://github.com/dani-garcia/vaultwarden/commit/a6c3bd6d1826fb527822df4a2655f34fd440d7d4) Update rust docker version (#7689)

## 变更文件（前 200 个）

- modified `.github/workflows/build.yml` (+1/-1)
- modified `.github/workflows/hadolint.yml` (+3/-3)
- modified `.github/workflows/release.yml` (+2/-2)
- modified `.github/workflows/trivy.yml` (+1/-1)
- modified `.github/workflows/typos.yml` (+1/-1)
- modified `.github/workflows/zizmor.yml` (+1/-1)
- modified `.pre-commit-config.yaml` (+1/-1)
- modified `Cargo.lock` (+320/-259)
- modified `Cargo.toml` (+18/-18)
- modified `docker/DockerSettings.yaml` (+1/-1)
- modified `docker/Dockerfile.alpine` (+4/-4)
- modified `docker/Dockerfile.debian` (+1/-1)
- modified `macros/Cargo.toml` (+1/-1)
- modified `rust-toolchain.toml` (+1/-1)
- modified `src/api/admin.rs` (+2/-2)
- modified `src/api/web.rs` (+0/-3)
- modified `src/config.rs` (+1/-1)
- modified `src/main.rs` (+3/-5)
- modified `src/static/scripts/admin_organizations.js` (+6/-5)
- modified `src/static/scripts/admin_users.js` (+35/-41)
- modified `src/static/scripts/datatables.css` (+95/-65)
- modified `src/static/scripts/datatables.js` (+12824/-14248)
- removed `src/static/scripts/jquery-4.0.0.slim.js` (+0/-6856)
- modified `src/static/templates/admin/organizations.hbs` (+1/-2)
- modified `src/static/templates/admin/users.hbs` (+3/-4)
- modified `src/util.rs` (+1/-4)

## 可能影响协议/客户端兼容性的文件

- `src/api/admin.rs`
- `src/api/web.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
