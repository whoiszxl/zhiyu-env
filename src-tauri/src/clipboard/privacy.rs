/// 敏感内容过滤器：识别并跳过密码、私钥、Token 等不应记录的内容。
pub(crate) fn is_sensitive(content: &str) -> bool {
    if content.is_empty() {
        return true;
    }
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();

    // 私钥
    if lower.contains("begin private key")
        || lower.contains("begin rsa private key")
        || lower.contains("begin ec private key")
        || lower.contains("begin openssh private key")
    {
        return true;
    }

    // JWT
    if looks_like_jwt(trimmed) {
        return true;
    }

    // 单独复制的验证码（纯数字 / 字母数字，4~8 位）
    if looks_like_verification_code(trimmed) {
        return true;
    }

    // 常见云服务 key / secret
    if lower.starts_with("ak-")
        || lower.starts_with("sk-")
        || lower.starts_with("key-")
        || lower.starts_with("api-")
        || lower.starts_with("token ")
        || lower.starts_with("bearer ")
        || (lower.starts_with("secret") && lower.contains('='))
    {
        return true;
    }

    // 密码字段特征：单行、带 password / passwd / pwd 等关键词
    if trimmed.lines().count() == 1
        && (lower.contains("password:")
            || lower.contains("password=")
            || lower.contains("passwd:")
            || lower.contains("passwd=")
            || lower.contains("pass:")
            || lower.contains("pwd:")
            || lower.contains("pwd=")
            || lower.contains("secret:")
            || lower.contains("secret="))
    {
        return true;
    }

    false
}

fn looks_like_jwt(text: &str) -> bool {
    // JWT 格式: header.payload.signature，三段 base64url，用 . 连接
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

fn looks_like_verification_code(text: &str) -> bool {
    let len = text.chars().count();
    if !(4..=8).contains(&len) {
        return false;
    }
    // 纯数字验证码
    if text.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // 字母数字混合验证码
    let alnum_count = text.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    alnum_count == len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_private_keys() {
        assert!(is_sensitive("-----BEGIN PRIVATE KEY-----"));
        assert!(is_sensitive("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(is_sensitive("-----BEGIN EC PRIVATE KEY-----"));
        assert!(is_sensitive("-----BEGIN OPENSSH PRIVATE KEY-----"));
    }

    #[test]
    fn skips_jwt() {
        assert!(is_sensitive(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"
        ));
    }

    #[test]
    fn skips_verification_codes() {
        assert!(is_sensitive("123456"));
        assert!(is_sensitive("A1B2C3"));
    }

    #[test]
    fn skips_api_keys() {
        assert!(is_sensitive("sk-proj-abc123"));
        assert!(is_sensitive("Bearer eyJhbGciOi..."));
        assert!(is_sensitive("token abc123def456"));
    }

    #[test]
    fn skips_password_lines() {
        assert!(is_sensitive("password: hunter2"));
        assert!(is_sensitive("PASSWORD=supersecret"));
        assert!(is_sensitive("pwd: 123456"));
    }

    #[test]
    fn keeps_normal_text() {
        assert!(!is_sensitive("hello world"));
        assert!(!is_sensitive("SELECT * FROM users"));
        assert!(!is_sensitive("http://localhost:8080"));
        assert!(!is_sensitive("redis://127.0.0.1:6379"));
    }
}
