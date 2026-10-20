//! Tests of the `privacy` module. Every test names the requirement it
//! verifies. Every sample is synthetic.

use super::*;

/// REQ-PRIV-002: every pattern matches a synthetic positive and not a
/// near miss; the finding names the kind and the character position.
#[test]
fn req_priv_002_patterns_match_positives_not_near_misses() {
    let positives: &[(SecretKind, &str)] = &[
        (SecretKind::AwsAccessKey, "AKIAIOSFODNN7EXAMPLE"),
        (
            SecretKind::GithubToken,
            &format!("ghp_{}", "A1b2C3d4E5f6G7h8I9j0".repeat(2)),
        ),
        (
            SecretKind::GithubToken,
            &format!("github_pat_{}", "11AAAAAAA0".repeat(3)),
        ),
        (
            SecretKind::SlackToken,
            "xoxb-0000000000-1111111111-AbCdEfGhIjKl",
        ),
        (
            SecretKind::GoogleApiKey,
            &format!("AIza{}", "Sy0123456789abcdefghijklmnopqrstuvw"),
        ),
        (
            SecretKind::StripeKey,
            &format!("sk_test_{}", "4eC39HqLyjWDarjtT1zdp7dc"),
        ),
        (
            SecretKind::OpenaiKey,
            &format!("sk-{}", "proj_abcdefghijklmnopqrstuvwxyz0123"),
        ),
        (SecretKind::PrivateKey, "-----BEGIN RSA PRIVATE KEY-----"),
        (
            SecretKind::Jwt,
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        ),
        (
            SecretKind::BearerHeader,
            "Bearer abcdefghijklmnopqrstuvwxyz",
        ),
        (
            SecretKind::BasicAuthUrl,
            "https://user:pass@example.org/path",
        ),
    ];
    for (kind, sample) in positives {
        let text = format!("query {sample} tail");
        let findings = scan(&text);
        assert_eq!(findings.len(), 1, "{sample}: {findings:?}");
        assert_eq!(findings[0].kind, *kind, "{sample}");
        assert_eq!(findings[0].start, 6, "{sample}");
    }
    let near_misses = [
        "AKIA1234",
        "ghp_short",
        "xoxz-not-a-slack-token-000",
        "AIzaTooShort",
        "sk_live_short",
        "sk-short",
        "BEGIN PRIVATE KEY",
        "eyJ.eyJ.sig",
        "bearer of bad news",
        "https://example.org/user:pass",
        "rust async await",
        "sha256 d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
    ];
    for sample in near_misses {
        assert!(scan(sample).is_empty(), "{sample}: {:?}", scan(sample));
    }
}

/// REQ-PRIV-002 and LIM-P1: each mode behaves as documented; the value
/// never appears in a verdict except as the masked text.
#[test]
fn req_priv_002_modes() {
    let text = "find AKIAIOSFODNN7EXAMPLE and ghp_A1b2C3d4E5f6G7h8I9j0A1b2C3d4E5f6G7h8I9j0 now";
    assert_eq!(check(text, SecretDetection::Off), Verdict::Clean);
    assert_eq!(
        check("plain query", SecretDetection::Refuse),
        Verdict::Clean
    );
    match check(text, SecretDetection::Refuse) {
        Verdict::Refuse(finding) => {
            assert_eq!(finding.kind, SecretKind::AwsAccessKey);
            assert_eq!(finding.start, 5);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(check(text, SecretDetection::Warn), Verdict::Warned(2));
    match check(text, SecretDetection::Mask) {
        Verdict::Masked { text, count } => {
            assert_eq!(text, "find [redacted] and [redacted] now");
            assert_eq!(count, 2);
        }
        other => panic!("{other:?}"),
    }
}

/// REQ-PRIV-004: positions count characters, not bytes, so a multi-byte
/// prefix does not shift them.
#[test]
fn req_priv_004_positions_are_character_offsets() {
    let findings = scan("日本語 AKIAIOSFODNN7EXAMPLE");
    assert_eq!(findings[0].start, 4);
    assert_eq!(findings[0].end, 24);
}

/// REQ-PRIV-004 and REQ-SEC-020: `Sensitive` has no `Display` and its
/// `Debug` shows the length only.
#[test]
fn req_priv_004_sensitive_debug_shows_no_content() {
    let s = Sensitive::new("secret query text");
    assert_eq!(format!("{s:?}"), "Sensitive(17 chars)");
    assert_eq!(s.expose(), "secret query text");
}
