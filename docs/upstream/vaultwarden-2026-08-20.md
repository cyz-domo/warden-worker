# Vaultwarden 上游变更摘要（2026-08-20）

- 上游仓库：[dani-garcia/vaultwarden](https://github.com/dani-garcia/vaultwarden)
- 对比区间：`0cefa4cca7` → `46d71107f5`（[查看完整 compare](https://github.com/dani-garcia/vaultwarden/compare/0cefa4cca7c9f2a5579dd290f78193b543818c51...46d71107f5094460dd5ecbe1dbac6e6c71e5189a)）
- 提交数：2；变更文件数：4
- 生成时间：2026-08-24 03:14 UTC

> 本 PR 仅为变更摘要，不包含代码改动。请评估后只移植适用于本 Cloudflare Worker（D1 + Durable Objects）实现的协议/API/安全相关改动，并保留 KDF、Send、2FA、白名单及部署逻辑。

## 提交列表

- [`9e78911a`](https://github.com/dani-garcia/vaultwarden/commit/9e78911a2f46818cd98582b5006b9836b46ad912) Fix sendmail executable permission check (#7483)
- [`46d71107`](https://github.com/dani-garcia/vaultwarden/commit/46d71107f5094460dd5ecbe1dbac6e6c71e5189a) add dummy revisionDate (#7608)

## 变更文件（前 200 个）

- modified `Cargo.lock` (+19/-0)
- modified `Cargo.toml` (+1/-0)
- modified `src/config.rs` (+2/-5)
- modified `src/db/models/org_policy.rs` (+1/-0)

## 可能影响协议/客户端兼容性的文件

- `src/db/models/org_policy.rs`

## 建议动作

- [ ] 对照上面的文件评估是否影响 Bitwarden 协议/客户端兼容性
- [ ] 需要时新增 D1 增量 migration（`migrations/`）
- [ ] 验证 `cargo fmt -- --check`、`cargo check` 和 WASM worker 构建
- [ ] 如需跟进，另开独立 PR 只移植兼容改动，不直接合并上游
