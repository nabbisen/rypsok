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
            // Digits, digits, a 24-character secret (RFC 020: the real shape).
            &cat(&[
                "xoxb", "-123456", "789012-", "1234567", "89012-A", "bCdEfGh", "IjKlMnO",
                "pQrStUv", "Wx",
            ]),
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
            // The legacy key: 48 base-62 characters (RFC 020).
            &format!("sk-{}A1b2C3d4", "A1b2C3d4E5f6G7h8I9j0".repeat(2)),
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

// --- the corpus of RFC 020 (DAT-15, handoff 020n) ----------------------------
//
// The corpus is the specification of the secret check. Every key in it is
// made up and obviously fake (a pattern of the documented shape, not an
// issued credential). The first list is ordinary text that the check must
// let through, the second is credential-shaped text that it must refuse,
// with the kind it must name. A rule of `privacy.rs` that is not decided by
// a line here has no business being there.

/// Joins the pieces of a corpus string. Every string of the corpus that has
/// the shape of a credential is written as pieces that do not have it, so
/// that the source text holds no literal a secret scanner (GitHub push
/// protection) would take for one; the test needs the string at run time,
/// not in the source (task 027).
fn cat(pieces: &[&str]) -> String {
    pieces.concat()
}

/// Ordinary text that the check refused before RFC 020 (the first nine,
/// found by audit 020c) or that is near a pattern's prefix (the rest).
fn ordinary() -> Vec<String> {
    vec![
        // The audit's nine.
        cat(&[
            "css cla", "ss sk-f", "olding-", "cube-sp", "inner-c", "hild",
        ]),
        cat(&["sk-hyni", "x-semic", "onducto", "r-roadm", "ap-2026"]),
        cat(&[
            "bearer ", "authent", "ication", "-and-au", "thoriza", "tion-fl", "ow",
        ]),
        cat(&["ASIAPAC", "IFICTRA", "DEZONE"]),
        "clone with ssh://git:@github.com/example/project.git".to_owned(),
        "https://example.org:8080@mirror.example/path".to_owned(),
        cat(&[
            "eyJustA", "nExampl", "eOfAVer", "yLongWo", "rd.seco", "ndVeryL", "ongWord", "InTheTe",
            "xt.thir", "dVeryLo", "ngWordH", "ereToo",
        ]),
        cat(&[
            "eyJsonS", "chemaVa", "lidatio", "nLibrar", "yName.a", "notherE", "xtremel", "yLongId",
            "entifie", "r.yetAn", "otherLo", "ngIdent", "ifierNa", "me",
        ]),
        cat(&[
            "eyJavas", "criptRu", "ntimeEn", "vironme", "ntSetup", ".config", "uration", "Managem",
            "entSyst", "em.depl", "oymentP", "ipeline", "Stages",
        ]),
        cat(&[
            "sk_test", "_config", "uration", "forthes", "taginge", "nvironm", "ent",
        ]),
        cat(&["xoxb", "-featur", "e-branc", "h-namin", "g-conve", "ntion"]),
        // Prefixes in prose.
        cat(&[
            "the bea", "rer of ", "this le", "tter is", " to be ", "admitte", "d",
        ]),
        cat(&[
            "bearer ", "interna", "tionali", "zation-", "and-loc", "alizati", "on-supp", "ort-not",
            "es",
        ]),
        cat(&[
            "a short", " ghp_ p", "refix a", "nd an n", "pm_ pre", "fix are", " docume", "nted in",
            " the gu", "ide",
        ]),
        cat(&[
            "AKIA is", " the pr", "efix of", " an acc", "ess key", " id, AS", "IA the ", "prefix ",
            "of a te", "mporary", " one",
        ]),
        cat(&[
            "AKIA ke", "ys are ", "rotated", " every ", "ninety ", "days",
        ]),
        cat(&[
            "AKIAIOS", "FODNN7 ", "is too ", "short t", "o be a ", "key id",
        ]),
        // Prefixes of the temporary and the older id with a word, not a key: the
        // digit is asked for these two (RFC 020, N2 of the 020n review).
        cat(&["A3TXPAC", "IFICTRA", "DEZONE"]),
        cat(&["hf_ and", " glpat", "- are t", "oken pr", "efixes"]),
        cat(&[
            "SG. and", " SK and", " sk- an", "d xoxb", "- are p", "refixes",
        ]),
        // Words near the new formats.
        "password reset instructions for the staff".to_owned(),
        "my password is stored in the vault".to_owned(),
        "password=configuration".to_owned(),
        "secret=true".to_owned(),
        "secret: hidden".to_owned(),
        "password=********".to_owned(),
        "password=<your-password>".to_owned(),
        "password=${PASSWORD_FROM_ENV}".to_owned(),
        "Authorization: Basic authentication is explained on this page".to_owned(),
        cat(&["Account", "Key=<yo", "ur-acco", "unt-key", ">"]),
        "aws_secret_access_key = <your key here>".to_owned(),
        "https://example.org:443@mirror.example/".to_owned(),
        // Each of these has the pattern of a rule and fails its confirmation, so
        // that the confirmation is decided by a line (the mutation run of
        // `evidence/020n/mutate-privacy.py` loosens each in turn).
        cat(&[
            "a wrapp", "ed line", ": ghp_a", "bcdefgh", "ij klmn", "opqrst",
        ]),
        cat(&[
            "sk-inte", "rnation", "alizati", "onandlo", "calizat", "ionsupp", "ortdocs",
        ]),
        cat(&[
            "sk-proj", "-roadma", "p-for-t", "he-spri", "ng-rele", "ase-of-", "the-new", "-planne",
            "r",
        ]),
        cat(&[
            "npm_abc", "defghij", "klmnopq", "rstuvwx", "yzabcde", "fghij",
        ]),
        cat(&["glpat", "-token-", "format-", "documen", "tation-", "page"]),
        cat(&["glpat", "-abcdef", "ghijklm", "nopqrst", "uvwxyz"]),
        cat(&["hf_abcd", "efghijk", "lmnopqr", "stuvwxy", "zabcdef", "gh"]),
        cat(&[
            "ratio 1", "2345678", "9:abcde", "fghijkl", "mnopqrs", "tuvwxyz", "abcdefg", "hi",
        ]),
        cat(&["SK12345", "6789012", "3456789", "0123456", "789012"]),
        // Near misses of Slack and OpenAI shapes (they were in the first test).
        cat(&[
            "xoxb", "-000000", "0000-11", "1111111", "1-AbCdE", "fGhIjKl",
        ]),
        cat(&["sk-proj", "_abcdef", "ghijklm", "nopqrst", "uvwxyz0", "123"]),
        // Hex and digits that are not credentials.
        "sha256 d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592".to_owned(),
        "SK 12345".to_owned(),
        cat(&["invoice", " 123456", "789:202", "6 total"]),
    ]
}

/// Credential-shaped text that was passed before (the first fourteen kinds,
/// found by audit 020c) or that was refused already, with the kind named.
fn credentials() -> Vec<(&'static str, String, SecretKind)> {
    let aws = cat(&["AKIAIOS", "FODNN7E", "XAMPLE"]);
    vec![
        // Passed before RFC 020.
        (
            "npm",
            cat(&[
                "npm_fjk", "kmNN0id", "HqHVRSG", "hlCWmuP", "GluuDV1", "vD4np",
            ]),
            SecretKind::NpmToken,
        ),
        (
            "gitlab",
            cat(&["glpat", "-Hhbetn", "HEohzxc", "fFi1YmL"]),
            SecretKind::GitlabToken,
        ),
        (
            "hugging face",
            cat(&["hf_sqcK", "gvnBWoW", "EbPZLZF", "kUZciGg", "TzgSHgM", "wP"]),
            SecretKind::HuggingfaceToken,
        ),
        (
            "sendgrid",
            cat(&[
                "SG.qwQh", "hBw2gTG", "CP5xT1F", "9imp.MY", "hPUVKQy", "pJJraSS", "SEvJwR0",
                "5eya0z4", "wUf_D3j", "ZzXXlX",
            ]),
            SecretKind::SendgridKey,
        ),
        (
            "telegram bot",
            cat(&[
                "1234567", "89:3voi", "zpLuZ_9", "hLiF4KM", "2hpch7x", "oNsTSQj", "lkE",
            ]),
            SecretKind::TelegramBotToken,
        ),
        (
            "twilio",
            cat(&["SKde30e", "bad5b7f", "b2fa58e", "f7dd0b8", "e6522a"]),
            SecretKind::TwilioKey,
        ),
        (
            "aws secret access key",
            "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_owned(),
            SecretKind::AwsSecretKey,
        ),
        (
            "aws secret access key, env style",
            "AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_owned(),
            SecretKind::AwsSecretKey,
        ),
        (
            "password=",
            "password=s3cretPassw0rd".to_owned(),
            SecretKind::PasswordAssignment,
        ),
        (
            "passwd=",
            "passwd=Hunter22Hunter22".to_owned(),
            SecretKind::PasswordAssignment,
        ),
        (
            "secret= (hex)",
            "secret=9f8e7d6c5b4a3210".to_owned(),
            SecretKind::PasswordAssignment,
        ),
        (
            "client_secret=",
            "client_secret=Zq7!mK2vXw9p".to_owned(),
            SecretKind::PasswordAssignment,
        ),
        (
            "Authorization: Basic",
            "Authorization: Basic ZmFrZXVzZXI6ZmFrZXBhc3N3b3JkMQ==".to_owned(),
            SecretKind::BasicAuthHeader,
        ),
        (
            "azure AccountKey=",
            cat(&[
                "Default", "Endpoin", "tsProto", "col=htt", "ps;Acco", "untName", "=fakeac",
                "count;A", "ccountK", "ey=AAzR", "gpk1ut0", "ZSshwxO", "fjqFp/6", "8dVksMX",
                "+gYAxqR", "H9wElDL", "2F0ylW7", "+phR9PX", "JaTzF6W", "WHyQL9Z", "OjPqVdP",
                "ny/sw==",
            ]),
            SecretKind::AzureAccountKey,
        ),
        (
            "Bearer%20",
            cat(&[
                "Authori", "zation=", "Bearer%", "20Zk3Qx", "9Lm2Vb7", "Nc4Rt8W", "y1Hd6Js",
                "5Pf0Ga",
            ]),
            SecretKind::BearerHeader,
        ),
        (
            "aws key with a zero width space inside",
            format!("{}\u{200B}{}", &aws[..4], &aws[4..]),
            SecretKind::AwsAccessKey,
        ),
        (
            "lowercase aws key",
            aws.to_lowercase(),
            SecretKind::AwsAccessKey,
        ),
        // About 1 in 27 real ids has no digit in its tail; the AKIA pattern
        // decides, a digit is not asked for (RFC 020, N2 of the 020n review).
        (
            "aws key id without a digit in the tail",
            cat(&["AKIAJKL", "MNOPQRS", "TUVWXY"]),
            SecretKind::AwsAccessKey,
        ),
        (
            "aws key id without a digit, lower case",
            cat(&["akiajkl", "mnopqrs", "tuvwxy"]),
            SecretKind::AwsAccessKey,
        ),
        (
            "aws temporary key id with a digit",
            cat(&["ASIAIOS", "FODNN7E", "XAMPLE"]),
            SecretKind::AwsAccessKey,
        ),
        (
            "github token split by a space",
            cat(&[
                "ghp_SVw", "8BoQgHI", "v9nVWO4", "7 xlefH", "NabpesA", "g7tzR2",
            ]),
            SecretKind::GithubToken,
        ),
        // Refused before, and still.
        ("aws", aws.to_owned(), SecretKind::AwsAccessKey),
        (
            "github",
            cat(&[
                "ghp_SVw", "8BoQgHI", "v9nVWO4", "7xlefHN", "abpesAg", "7tzR2",
            ]),
            SecretKind::GithubToken,
        ),
        (
            "slack",
            cat(&[
                "xoxb", "-123456", "789012-", "1234567", "89012-f", "VkDVpgQ", "ninapdW",
                "JULmzNC", "t0",
            ]),
            SecretKind::SlackToken,
        ),
        (
            "stripe",
            cat(&["sk_live", "_nlNAyf", "hrL83bF", "GPznoF6", "L1UH"]),
            SecretKind::StripeKey,
        ),
        (
            "openai",
            cat(&[
                "sk-GBTn", "gybjkOA", "uk2LgT1", "CjzVysY", "b4POsOE", "ppAdM9m", "vOtOiNo", "vz",
            ]),
            SecretKind::OpenaiKey,
        ),
        (
            "openai project",
            cat(&[
                "sk-proj", "-PU7aCo", "zThSl_K", "xUbkxim", "0Xy5oMX", "lyODKGb", "w4jeCLW",
                "jLjRkVJ", "ydVnNIH", "ShLJrxK", "Fs",
            ]),
            SecretKind::OpenaiKey,
        ),
        (
            "anthropic",
            cat(&[
                "sk-ant-", "api03-1", "xKAnqCn", "Urbeobe", "4-RP7-h", "jGnMaAU", "0e7QvWC",
                "66swX3R", "wKn26gt", "MggB6rX", "LK5rEpB", "KEFy6qZ", "NjtgXR9", "hsG2e3J",
                "LyQwu",
            ]),
            SecretKind::OpenaiKey,
        ),
        (
            "jwt",
            cat(&[
                "eyJhbGc", "iOiJIUz", "I1NiIsI", "nR5cCI6", "IkpXVCJ", "9.eyJzd", "WIiOiJm",
                "YWtlLXV", "zZXIiLC", "JpYXQiO", "jE3MDAw", "MDAwMDB", "9.7fyq7", "CRgduF6",
                "TcSNRd5", "UvCwZFV", "5GR6n8r", "TETBl1r", "X_m",
            ]),
            SecretKind::Jwt,
        ),
        (
            "bearer",
            cat(&[
                "Authori", "zation:", " Bearer", " Zk3Qx9", "Lm2Vb7N", "c4Rt8Wy", "1Hd6Js5",
                "Pf0Ga",
            ]),
            SecretKind::BearerHeader,
        ),
        (
            "url with a password",
            "https://deploy:Tr0ub4dor3@mirror.example/repo.git".to_owned(),
            SecretKind::BasicAuthUrl,
        ),
        (
            "url with a numeric password",
            "ftp://backup:20261103@files.example/".to_owned(),
            SecretKind::BasicAuthUrl,
        ),
        (
            "private key",
            "-----BEGIN OPENSSH PRIVATE KEY-----".to_owned(),
            SecretKind::PrivateKey,
        ),
    ]
}

/// REQ-PRIV-002 (RFC 020, DAT-15): the ordinary strings pass, in a text of
/// their own and inside a sentence.
#[test]
fn req_priv_002_the_corpus_of_ordinary_text_passes() {
    let mut refused = Vec::new();
    for sample in ordinary() {
        for text in [sample.clone(), format!("how do I use {sample} in a build?")] {
            let findings = scan(&text);
            if !findings.is_empty() {
                refused.push(format!("{text:?}: {findings:?}"));
            }
        }
    }
    assert!(
        refused.is_empty(),
        "{} refused:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// REQ-PRIV-002 (RFC 020, DAT-15): the credential-shaped strings are
/// refused with the kind named, alone and inside a sentence, and the finding
/// covers the credential from its first to its last character (the
/// positions are in characters of the original text, a zero width character
/// inside included).
#[test]
fn req_priv_002_the_corpus_of_credentials_is_refused() {
    let mut wrong = Vec::new();
    for (label, sample, kind) in credentials() {
        let text = format!("please use {sample} for it");
        let findings = scan(&text);
        match findings.as_slice() {
            [f] if f.kind == kind => {
                let chars: Vec<char> = text.chars().collect();
                let covered: String = chars[f.start as usize..f.end as usize].iter().collect();
                if !sample.contains(&covered) && !covered.contains(&sample) {
                    wrong.push(format!(
                        "{label}: finding covers {covered:?}, sample {sample:?}"
                    ));
                }
            }
            other => wrong.push(format!("{label}: expected {kind:?}, found {other:?}")),
        }
        if check(&sample, SecretDetection::Refuse) == Verdict::Clean {
            wrong.push(format!("{label}: alone, it passes"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// REQ-PRIV-002 (RFC 020, DAT-15): mask mode replaces the whole credential,
/// including a zero width character inside it, and nothing around it.
#[test]
fn req_priv_002_mask_covers_a_credential_with_an_invisible_character_inside() {
    let aws = "AKIA\u{200B}IOSFODNN7EXAMPLE";
    let text = format!("key {aws} end");
    match check(&text, SecretDetection::Mask) {
        Verdict::Masked { text, count } => {
            assert_eq!(text, "key [redacted] end");
            assert_eq!(count, 1);
        }
        other => panic!("{other:?}"),
    }
}

/// REQ-PRIV-002 (RFC 020, TST-7): the overlap rule of `scan`. Two findings
/// that overlap are one finding, the one the table lists first (the more
/// specific pattern comes before the general `Bearer` one); findings that
/// only touch, or are apart, are both kept; mask mode never slices backwards.
#[test]
fn req_priv_002_overlapping_findings_are_one_finding() {
    let github = cat(&[
        "ghp_SVw", "8BoQgHI", "v9nVWO4", "7xlefHN", "abpesAg", "7tzR2",
    ]);
    let text = format!("Bearer {github}");
    let findings = scan(&text);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].kind, SecretKind::GithubToken);
    assert_eq!((findings[0].start, findings[0].end), (7, 47));
    match check(&text, SecretDetection::Mask) {
        Verdict::Masked { text, count } => {
            assert_eq!(text, cat(&["Bearer ", "[redact", "ed]"]));
            assert_eq!(count, 1);
        }
        other => panic!("{other:?}"),
    }
    // Apart: two findings, in order, both masked.
    let apart = format!("AKIAIOSFODNN7EXAMPLE and {github}");
    let findings = scan(&apart);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert!(findings[0].end < findings[1].start);
    match check(&apart, SecretDetection::Mask) {
        Verdict::Masked { text, count } => {
            assert_eq!(text, "[redacted] and [redacted]");
            assert_eq!(count, 2);
        }
        other => panic!("{other:?}"),
    }
}

/// REQ-PRIV-002 (RFC 020, TST-7): the predicate itself, at its edges
/// (the mutants of audit 020d that survived all sat on this line).
#[test]
fn req_priv_002_the_overlap_predicate_at_its_edges() {
    let found = Finding {
        kind: SecretKind::Jwt,
        start: 10,
        end: 20,
    };
    for (start, end, overlaps_it) in [
        (0, 10, false),  // touches at the start
        (20, 30, false), // touches at the end
        (0, 5, false),
        (25, 30, false),
        (0, 11, true), // one character in
        (19, 30, true),
        (12, 15, true), // inside
        (5, 25, true),  // around
        (10, 20, true), // the same
    ] {
        assert_eq!(overlaps(&found, start, end), overlaps_it, "{start}..{end}");
    }
}

// --- Mutants the audit run left alive (RFC 020, TST-7) ----------------------
// Every sample is built here at run time from pieces; none is a real
// credential.

/// Base64 of `bytes`, standard or URL-safe alphabet, without padding.
fn b64(bytes: &[u8], url_safe: bool) -> String {
    let alphabet: Vec<char> = ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .chain(if url_safe { ['-', '_'] } else { ['+', '/'] })
        .collect();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(alphabet[((n >> (18 - 6 * i)) & 63) as usize]);
        }
    }
    out
}

fn kinds(text: &str) -> Vec<SecretKind> {
    scan(text).into_iter().map(|f| f.kind).collect()
}

/// REQ-PRIV-002: the "mixed" rules ask for a digit, an upper-case and a
/// lower-case letter, each; a key body missing one of the three is a word or
/// an identifier, not a key (mutants of `has_lower` and of the `&&` between
/// the three were missed).
#[test]
fn req_priv_002_a_mixed_key_needs_a_digit_an_upper_and_a_lower_case_letter() {
    let key = |body: &str| format!("sk-{body}");
    let mixed = "A1b2C3d4E5".repeat(5);
    assert_eq!(kinds(&key(&mixed)), [SecretKind::OpenaiKey]);
    // No lower-case letter.
    assert!(kinds(&key(&"A1B2C3D4E5".repeat(5))).is_empty());
    // Lower-case letters only.
    assert!(kinds(&key(&"abcdefghij".repeat(5))).is_empty());
    // No digit.
    assert!(kinds(&key(&"AbCdEfGhIj".repeat(5))).is_empty());
    // No upper-case letter.
    assert!(kinds(&key(&"a1b2c3d4e5".repeat(5))).is_empty());
}

/// REQ-PRIV-002: a JWT is a header that is a JSON object naming `alg` or
/// `enc` and a payload that is a JSON object; a segment that merely starts
/// with `{` or merely ends with `}` is not an object (mutants of
/// `is_json_object` and of the `&&` of the check were missed).
#[test]
fn req_priv_002_a_jwt_is_two_json_objects_and_the_header_names_its_algorithm() {
    let token = |header: &str, payload: &str| {
        format!(
            "{}.{}.{}",
            b64(header.as_bytes(), true),
            b64(payload.as_bytes(), true),
            "signature0123456789"
        )
    };
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = r#"{"sub":"1234567890","name":"x"}"#;
    assert_eq!(kinds(&token(header, payload)), [SecretKind::Jwt]);
    assert_eq!(
        kinds(&token(r#"{"enc":"A128GCM","x":1}"#, payload)),
        [SecretKind::Jwt]
    );
    // A header that is an object but names neither `alg` nor `enc`.
    assert!(kinds(&token(r#"{"typ":"JWT","x":1234567890}"#, payload)).is_empty());
    // A header that names `alg` but is not a complete object.
    assert!(kinds(&token(r#"{"alg":"HS256","typ":"JWT""#, payload)).is_empty());
    // A payload that is an array, one that opens and does not close, and one
    // that closes and does not open.
    assert!(kinds(&token(header, "[1,2,3,4,5,6,7,8,9]")).is_empty());
    assert!(kinds(&token(header, r#"{"sub":"1234567890","x":["#)).is_empty());
    assert!(kinds(&token(header, r#""sub":"1234567890"}"#)).is_empty());
}

/// REQ-PRIV-002: the Basic payload is `user:password` with both parts, in
/// either base64 alphabet; the characters of both alphabets that are not
/// letters or digits decode (mutants of `decode_base64`'s alphabet arms and
/// of the emptiness checks were missed).
#[test]
fn req_priv_002_a_basic_header_carries_user_and_password_in_either_alphabet() {
    let header = |payload: &str, url_safe: bool| {
        format!("Authorization: Basic {}", b64(payload.as_bytes(), url_safe))
    };
    // Payloads whose encodings contain the 62nd and 63rd characters of each
    // alphabet: the samples must use all four marks, or they prove nothing.
    let payloads = ["user:~~~p>?", "user:~~~~~~", "user:>>>???", "user:p>?p>?"];
    let encodings: String = payloads
        .iter()
        .flat_map(|p| [b64(p.as_bytes(), false), b64(p.as_bytes(), true)])
        .collect();
    for mark in ['+', '/', '-', '_'] {
        assert!(encodings.contains(mark), "no sample encodes to {mark}");
    }
    for payload in payloads {
        for url_safe in [false, true] {
            assert_eq!(
                kinds(&header(payload, url_safe)),
                [SecretKind::BasicAuthHeader],
                "{payload} {url_safe}"
            );
        }
    }
    // Both parts are required.
    assert!(kinds(&header("user:", false)).is_empty());
    assert!(kinds(&header(":password", false)).is_empty());
    assert!(kinds(&header("nocolonhere", false)).is_empty());
}

/// REQ-PRIV-002: `user:12345@host` is credentials; only a dotted name with a
/// short number (`host.example:8080@other`) is a host and a port (a mutant of
/// the `&&` in `port_like` was missed).
#[test]
fn req_priv_002_a_short_number_after_a_plain_user_is_still_a_password() {
    assert_eq!(
        kinds("https://user:12345@example.org/"),
        [SecretKind::BasicAuthUrl]
    );
    assert_eq!(
        kinds("https://alice:8080@example.org/"),
        [SecretKind::BasicAuthUrl]
    );
    assert!(kinds("https://host.example:8080@other.example/").is_empty());
}

/// REQ-PRIV-002: a GitHub token wrapped at a space is found when the joined
/// text is 36 characters with a digit, an upper-case and a lower-case letter,
/// and only then (mutants of the `&&` chain of `Check::Split` were missed).
#[test]
fn req_priv_002_a_wrapped_github_token_needs_its_length_and_all_three_classes() {
    let wrapped = |first: &str, second: &str| format!("ghp_{first} {second}");
    let long_mixed = ("A1b2C3d4E5f6G7h8I9", "j0A1b2C3d4E5f6G7h8");
    assert_eq!(
        kinds(&wrapped(long_mixed.0, long_mixed.1)),
        [SecretKind::GithubToken]
    );
    // Long enough, no digit.
    assert!(kinds(&wrapped(&"AbCdEfGhIj".repeat(2), &"KlMnOpQrSt".repeat(2))).is_empty());
    // Long enough, no lower-case letter.
    assert!(kinds(&wrapped(&"A1B2C3D4E5".repeat(2), &"F6G7H8I9J0".repeat(2))).is_empty());
    // Long enough, no upper-case letter.
    assert!(kinds(&wrapped(&"a1b2c3d4e5".repeat(2), &"f6g7h8i9j0".repeat(2))).is_empty());
    // All three classes, too short.
    assert!(kinds(&wrapped("Abc1", "def2")).is_empty());
    assert!(kinds(&wrapped("Abcdef", "ghi")).is_empty());
}
