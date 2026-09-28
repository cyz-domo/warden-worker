# Vaultwarden 上游变更摘要（2026-09-25）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`cc67d644f6` → `061694d0cb`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/cc67d644f62605cb46f4d16c4a2eed1a861cc8bb...061694d0cb3bbf5d4c7e920c892824f0020cff83)）
- 提交数：3；变更文件数：8
- 生成时间：2026-09-28 08:39 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`d2660324`](https://github.com/dani-garcia/vaultwarden/commit/d2660324e67073a856f6664ae0eb10324b1c38e6) Fix revoked org members retaining access to org ciphers (#7554)
- [`32098ca7`](https://github.com/dani-garcia/vaultwarden/commit/32098ca7d1b94abce848b03e66545442a5d0e836) Require confirmed membership for user access checks, not for admin views (#7763)
- [`061694d0`](https://github.com/dani-garcia/vaultwarden/commit/061694d0cb3bbf5d4c7e920c892824f0020cff83) Fix cortex-a53 build issues when using xx-cargo (#7774)

## 变更文件（前 200 个）

- modified `docker/Dockerfile.debian` (+20/-4)
- modified `docker/Dockerfile.j2` (+10/-2)
- modified `src/api/core/ciphers.rs` (+8/-8)
- modified `src/api/core/organizations.rs` (+14/-8)
- modified `src/db/models/cipher.rs` (+20/-1)
- modified `src/db/models/collection.rs` (+17/-2)
- modified `src/db/models/group.rs` (+6/-1)
- modified `src/db/models/organization.rs` (+22/-58)

## 可能影响协议/客户端兼容性的文件

- `src/api/core/ciphers.rs`
- `src/api/core/organizations.rs`
- `src/db/models/cipher.rs`
- `src/db/models/collection.rs`
- `src/db/models/group.rs`
- `src/db/models/organization.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
