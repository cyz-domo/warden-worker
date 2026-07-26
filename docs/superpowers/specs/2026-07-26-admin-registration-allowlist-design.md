# 管理员后台与注册邮箱白名单设计

## 目标

增加一个受 `ADMIN_EMAIL` 保护的管理后台，用 D1 动态维护注册邮箱白名单，并提供全局用户统计。管理员不提供重置其他用户密码的功能，以避免破坏 Bitwarden 用户密钥和已有加密数据。

## 权限模型

- `ADMIN_EMAIL` 是唯一管理员邮箱，按规范化后的大小写不敏感邮箱匹配。
- 管理接口要求已登录 Bearer JWT，且 JWT 对应用户邮箱必须等于 `ADMIN_EMAIL`。
- Cloudflare Access 可在部署层进一步保护 `/admin.html`，但 Worker 仍执行应用层管理员校验，避免直接访问 Worker 域名绕过页面保护。
- 不新增管理员数据库角色，也不提供管理员修改或重置用户密码的接口。

## 注册白名单

新增 D1 表 `registration_allowlist`：

- `email`：规范化邮箱，主键；
- `enabled`：是否允许注册；
- `created_at`、`updated_at`：审计时间。

注册时读取该表：

1. 白名单没有任何记录时，允许所有邮箱注册；
2. 白名单存在记录时，只允许匹配且 `enabled = 1` 的邮箱；
3. 管理员邮箱不绕过白名单规则，仍按普通注册开关判断。

管理接口支持查询、新增、启用/禁用和删除白名单记录。邮箱统一 `trim` 后转小写，拒绝空值；同一邮箱重复新增采用更新启用状态的幂等行为。

## 用户统计

后台提供用户列表和汇总统计：

- 用户总数；
- 已启用 TOTP 的用户数；
- 白名单记录数及启用数；
- 用户的 ID、名称、邮箱、注册时间、更新时间和 TOTP 状态。

接口只返回管理所需的元数据，不返回 `master_password_hash`、`key`、`private_key`、JWT、TOTP 密钥或其他敏感字段。

## 页面与接口

新增 `/admin.html`，使用当前管理员登录后的 Bearer Token 调用 API，提供统计卡片、用户列表和白名单编辑区域。页面不保存密码或令牌到服务端。

新增管理员接口：

- `GET /api/admin/summary`：统计汇总；
- `GET /api/admin/users`：用户列表；
- `GET /api/admin/allowlist`：白名单列表；
- `POST /api/admin/allowlist`：新增或更新邮箱；
- `PATCH /api/admin/allowlist/{email}`：启用或禁用邮箱；
- `DELETE /api/admin/allowlist/{email}`：删除邮箱。

所有接口统一返回 JSON 错误，并复用现有 JWT Claims 认证。

## 数据库与部署

新增幂等 migration，使用 `CREATE TABLE IF NOT EXISTS`，不清空现有数据。测试和生产部署都通过对应 Wrangler 配置执行 migration。首次部署前若 migration 未自动应用，应先手动执行远程 migration。

## 验收标准

1. `ADMIN_EMAIL` 对应用户能访问 `/admin.html` 和全部管理接口。
2. 非管理员即使持有有效 JWT，也收到 403，不能读取用户或白名单数据。
3. 白名单为空时任意邮箱可注册。
4. 白名单非空时仅启用记录允许注册。
5. 管理员可以新增、启用、禁用、删除白名单邮箱，变更即时影响注册。
6. 用户统计不泄露密码哈希、密钥、令牌或 TOTP 密钥。
7. 不存在管理员重置用户密码的功能。
