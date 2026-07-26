# 验证器 App 二步验证设计

## 目标

让 Bitwarden Web Vault 的“验证器 App”原生管理流程可用，同时保留 `/demo.html` 的现有自定义配置流程。用户使用标准 TOTP 验证器生成 6 位验证码完成启用、登录和禁用。

## 范围

- 支持 `POST /api/two-factor/get-authenticator` 获取配置密钥。
- 支持 `PUT /api/two-factor/authenticator` 使用 `key`、`token`、`masterPasswordHash` 启用 TOTP。
- 支持 Web Vault 所需的响应字段，包括 `enabled`、`key` 和 `userVerificationToken`（无用户验证令牌时返回空值）。
- 支持 `DELETE /api/two-factor/authenticator` 禁用 TOTP，并校验主密码哈希及当前配置密钥。
- 保留 `/api/two-factor/authenticator/request`、`enable`、`disable` 及 `/demo.html` 入口。
- 登录时继续使用 provider `0` 和标准 TOTP 校验。

## 安全约束

- 只接收并比对客户端提供的 `masterPasswordHash`，不接收或保存明文主密码。
- TOTP 密钥继续使用 `TWO_FACTOR_ENC_KEY` 加密保存；未配置密钥时保持现有 `plain:` 兼容行为。
- 不记录主密码哈希、TOTP 密钥、验证码或令牌。
- 只接受标准 20 字节 Base32 TOTP 密钥和 6 位验证码。

## 兼容策略

原生 Web Vault 请求中的附加字段（例如 `userVerificationToken`）允许被反序列化但不改变现有主密码哈希校验逻辑。原生接口与自定义接口使用各自的 payload 类型，避免改变 `/demo.html` 的行为。

## 验收标准

1. 未启用时 Web Vault 能获取密钥并完成 TOTP 配置。
2. 输入有效验证码和正确主密码哈希后，PUT 返回成功且状态接口显示 provider `0`。
3. 登录接口要求并验证 TOTP；错误验证码不能登录。
4. Web Vault 能读取当前配置并成功禁用。
5. `/demo.html` 的生成、启用、禁用流程继续可用。
6. `cargo check` 及相关测试通过。
