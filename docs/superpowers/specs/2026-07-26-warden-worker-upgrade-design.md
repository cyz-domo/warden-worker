# Warden Worker 升级与发布设计

## 目标

在保留 Cloudflare Workers + D1 架构的前提下，升级项目所携带的 Bitwarden Web Vault，补齐当前移动端登录所需的兼容性，并建立“测试环境自动部署、生产环境手动发布”的交付通道。

## 非目标

- 不直接将官方 Vaultwarden 的 Rocket、Diesel、Tokio 服务编译为 WASM。
- 不在本阶段迁移到 VPS、Docker 或官方 Vaultwarden 原生运行时。
- 不自动重建或清空生产 D1。

## 方案

以当前 `afoim/warden-worker` 兼容层为基础，参考 Vaultwarden 和 Bitwarden 客户端的公开 API 行为，通过当前 Rust/Axum Worker 实现适配。Web Vault 作为静态资源随 Worker 部署，版本号在构建阶段统一校验。

部署分为两个 Cloudflare 环境：

- `test`：独立 Worker 与独立 D1；推送到 `develop` 后自动构建、初始化/迁移并部署。
- `production`：独立 Worker 与独立 D1；通过 GitHub Actions 的手动 workflow 发布，发布前执行检查，不自动覆盖数据库。

生产部署使用 GitHub Environments 保护，并将 Cloudflare API Token、Account ID、测试/生产 D1 ID 分开保存。工作流不把密钥写入仓库文件。

## 版本策略

以一个构建版本变量作为来源，统一生成/校验：

- `static/web-vault/version.json`
- `static/web-vault/vw-version.json`
- `/api/config` 的 `version`
- `/api/version`

构建失败条件包括两个静态版本文件不一致，或静态 Web Vault 版本与 Rust 服务端版本不一致。升级 Web Vault 时同步更新兼容性说明，并保留当前资源可回滚。

## 移动端兼容验证

增加不依赖真实账号的接口冒烟检查，覆盖配置探测、预登录、登录请求格式、错误响应格式及设备探测；对需要数据库/密钥的登录和同步流程，在测试 D1 中使用专用测试账号执行回归。重点验证：

- `/api/config`、`/api/alive`、`/api/version`
- `/identity/accounts/prelogin`
- `/identity/connect/token`
- `/api/sync`
- `/api/devices/knowndevice`
- refresh token、remember-device 和 TOTP 请求的字段与 Cookie

脚本只检查 HTTP 状态码、响应 JSON 结构和版本一致性，不记录密码、令牌或 D1 密钥。

## 数据库策略

测试数据库可全新初始化，因为当前没有生产数据。初始化 schema 与增量 migration 分开：

- schema 仅用于新建空数据库；
- migration 只使用 `CREATE TABLE IF NOT EXISTS`、`ALTER TABLE` 等可审计增量操作；
- 生产 workflow 不执行 schema 全量初始化；
- 后续生产发布前先执行显式 D1 备份步骤。

## 发布流程

```text
develop push
  -> Rust/WASM 构建
  -> Web Vault/API 版本检查
  -> API 冒烟测试
  -> 测试 D1 migration
  -> 自动部署 test Worker

GitHub Actions 手动发布
  -> checkout 指定 ref
  -> 完整构建与检查
  -> production D1 备份/增量 migration
  -> 手动确认后部署 production Worker
```

生产发布失败时保留旧 Worker 版本；本阶段不自动回滚 D1，因为数据库迁移必须设计为向前兼容。

## 验收标准

1. 测试环境能打开新版 Web Vault。
2. Web Vault 与 `/api/config`、`/api/version` 版本一致。
3. 官方最新移动 App 能完成服务器探测、预登录和登录流程，不因设备接口响应异常退出。
4. 推送 `develop` 能自动部署测试环境。
5. 生产环境只能通过手动 workflow 发布。
6. 测试与生产使用不同 Worker、D1 和 secrets。
7. CI 日志不输出密码、JWT、TOTP 或 Cloudflare token。
