//! Tests of the messages for a configuration that does not parse or does
//! not validate (RFC 020, DAT-11, DAT-12, DAT-14). Every test names the
//! requirement it verifies.

use super::super::*;

/// What a case of the table expects.
enum Expect {
    /// Parses and validates.
    Valid,
    /// Refused; each text must appear in the problems, together.
    Refused(&'static [&'static str]),
}

use Expect::{Refused, Valid};

/// The problems of a text: from `parse`, or else from `validate`.
fn problems(text: &str) -> Option<Vec<String>> {
    match Config::parse(text) {
        Err(error) => Some(error.problems),
        Ok(config) => config.validate().err().map(|e| e.problems),
    }
}

/// Words that name a Rust type or a serde message and mean nothing to the
/// person who wrote the file.
const FORBIDDEN: &[&str] = &[
    "u8",
    "u16",
    "u32",
    "u64",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "usize",
    "struct",
    "sequence",
    "invalid type",
    "invalid value",
    "at byte",
    "expected a boolean",
    "expected a string",
];

fn mentions_a_rust_type(message: &str) -> Option<&'static str> {
    FORBIDDEN.iter().copied().find(|word| {
        message
            .split(|c: char| !c.is_ascii_alphanumeric() && c != ' ')
            .any(|part| {
                if word.contains(' ') {
                    part.contains(word)
                } else {
                    part.split(' ').any(|w| w == *word)
                }
            })
    })
}

/// REQ-CFG-005 (RFC 020, DAT-11, area 3 of audit 020c): the invalid
/// configurations of the audit, rebuilt. A refusal names the key path in the
/// words of the file, says what was expected and what was found, gives the
/// line and the column, and never mentions a Rust type. (The first 30 rows
/// are the audit's table; the last six are the cases the audit's list did
/// not reach.)
#[test]
fn req_cfg_005_the_invalid_configurations_of_the_audit() {
    let cases: &[(&str, &str, Expect)] = &[
        (
            "wrong type",
            "config_version = 1\n[search]\nmax_results = \"five\"\n",
            Refused(&[
                "search.max_results: expected an integer from 1 to 20, found text (line 3, column 15)",
            ]),
        ),
        (
            "negative",
            "config_version = 1\n[search]\nmax_results = -1\n",
            Refused(&[
                "search.max_results: expected an integer from 1 to 20, found -1 (line 3, column 15)",
            ]),
        ),
        (
            "huge integer",
            "config_version = 1\n[search]\nmax_results = 99999999999999999999\n",
            Refused(&[
                "search.max_results: expected an integer from 1 to 20, found 99999999999999999999 (line 3, column 15)",
            ]),
        ),
        (
            "empty region",
            "config_version = 1\n[search]\nregion = \"\"\n",
            Refused(&["search.region: must be two uppercase letters"]),
        ),
        (
            "duplicate engine table",
            "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n[engines.w]\ntype = \"wikipedia\"\n",
            Refused(&["engines.w: defined more than once (line 5, column 10)"]),
        ),
        (
            "unknown engine type",
            "config_version = 1\n[engines.w]\ntype = \"nothing\"\ncategories = [\"general\"]\n",
            Refused(&["engines.w.type: no such driver"]),
        ),
        (
            "secret literal",
            "config_version = 1\n[engines.b]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key = \"abc\"\n",
            Refused(&["engines.b.api_key: give the secret by api_key_env or api_key_file"]),
        ),
        (
            "CRLF",
            "config_version = 1\r\n[search]\r\nmax_results = 5\r\n",
            Valid,
        ),
        ("BOM", "\u{FEFF}config_version = 1\n", Valid),
        (
            "no config_version",
            "[search]\nmax_results = 5\n",
            Refused(&["config_version: required; this release supports 1"]),
        ),
        (
            "config_version 2",
            "config_version = 2\n",
            Refused(&["config_version: only 1 is supported"]),
        ),
        (
            "typo key",
            "config_version = 1\n[search]\nmax_result = 5\n",
            Refused(&[
                "search.max_result: unknown key; expected one of languages, region, safe_search, max_results,",
                "(line 3, column 1)",
            ]),
        ),
        (
            "typo section",
            "config_version = 1\n[servce]\nx = 1\n",
            Refused(&[
                "servce: unknown key; expected one of config_version, service, search, fetch,",
                "(line 2, column 2)",
            ]),
        ),
        (
            "no category",
            "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = []\n",
            Refused(&["engines.w.categories: at least one category is required"]),
        ),
        (
            "uppercase engine id",
            "config_version = 1\n[engines.W]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
            Refused(&["engines.W: not an engine identifier"]),
        ),
        (
            "bind a host name",
            "config_version = 1\n[service.http]\nbind = \"localhost:8737\"\n",
            Refused(&["service.http.bind: must be an address and a port"]),
        ),
        (
            "non-loopback, no callers",
            "config_version = 1\n[service.http]\nbind = \"192.0.2.1:8737\"\n",
            Refused(&["service.http.bind: a non-loopback bind needs [service.http.auth] callers"]),
        ),
        (
            "a number as text",
            "config_version = 1\n[search]\ndeadline_ms = \"60\"\n",
            Refused(&[
                "search.deadline_ms: expected an integer from 500 to 10000, found text (line 3, column 15)",
            ]),
        ),
        (
            "a float for an integer",
            "config_version = 1\n[search]\ndeadline_ms = 2500.5\n",
            Refused(&[
                "search.deadline_ms: expected an integer from 500 to 10000, found 2500.5 (line 3, column 15)",
            ]),
        ),
        (
            "ports [0, 70000]",
            "config_version = 1\n[fetch]\nports = [0, 70000]\n",
            Refused(&[
                "fetch.ports[1]: expected an integer from 1 to 65535, found 70000 (line 3, column 13)",
            ]),
        ),
        (
            "allow rule with a scheme",
            "config_version = 1\n[egress]\nallow = [\"https://example.org\"]\n",
            Refused(&[
                "egress.allow[0]: not a network in CIDR notation, an address or a host pattern",
            ]),
        ),
        (
            "deny rule with a trailing dot",
            "config_version = 1\n[egress]\ndeny = [\"blocked.invalid.\"]\n",
            Refused(&[
                "egress.deny[0]: a host pattern must not end with a dot (write blocked.invalid, not blocked.invalid.)",
            ]),
        ),
        (
            "language EN",
            "config_version = 1\n[search]\nlanguages = [\"EN\"]\n",
            Refused(&[
                "search.languages[0]: expected a language tag (two or three lowercase letters, optionally a hyphen and a two-letter uppercase region, such as en or pt-BR), found text (line 3, column 14)",
            ]),
        ),
        (
            "region USA",
            "config_version = 1\n[search]\nregion = \"USA\"\n",
            Refused(&["search.region: must be two uppercase letters"]),
        ),
        (
            "empty file",
            "",
            Refused(&["config_version: required; this release supports 1"]),
        ),
        (
            "not TOML",
            "this is not toml\n",
            Refused(&["TOML syntax error: key with no value, expected `=` (line 1, column 6)"]),
        ),
        (
            "engine unknown key",
            "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\ncolour = \"red\"\n",
            Refused(&["engines.w.colour: not a key of driver wikipedia"]),
        ),
        (
            "bool as text",
            "config_version = 1\n[fetch]\nrobots_txt = \"yes\"\n",
            Refused(&["fetch.robots_txt: expected true or false, found text (line 3, column 14)"]),
        ),
        (
            "scraping engine, switch off",
            "config_version = 1\n[engines.d]\ntype = \"duckduckgo\"\ncategories = [\"general\"]\n",
            Refused(&["engines.d: type \"duckduckgo\" scrapes a human-facing interface"]),
        ),
        (
            "duplicate caller id",
            "config_version = 1\n[service.http]\nbind = \"192.0.2.1:8737\"\n[[service.http.auth.callers]]\nid = \"a\"\ntoken_env = \"X\"\n[[service.http.auth.callers]]\nid = \"a\"\ntoken_env = \"Y\"\n",
            Refused(&["service.http.auth.callers[1].id: duplicate"]),
        ),
        // Beyond the audit's table.
        (
            "a table where a table is meant, found a number",
            "config_version = 1\nsearch = 5\n",
            Refused(&["search: expected a table, found 5 (line 2, column 10)"]),
        ),
        (
            "an unknown word for a setting",
            "config_version = 1\n[privacy]\nsecret_detection = \"maybe\"\n",
            Refused(&[
                "privacy.secret_detection: expected one of off, warn, mask, refuse, found an unknown word (line 3, column 20)",
            ]),
        ),
        (
            "an unknown category",
            "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"foo\"]\n",
            Refused(&[
                "engines.w.categories[0]: expected one of general, news, science, code, found an unknown word (line 4, column 15)",
            ]),
        ),
        (
            "text where a list is meant",
            "config_version = 1\n[egress]\nallow = \"10.0.0.0/8\"\n",
            Refused(&[
                "egress.allow: expected a list of text rules, such as [\"10.0.0.0/8\"], found text (line 3, column 9)",
            ]),
        ),
        (
            "a missing type",
            "config_version = 1\n[engines.w]\ncategories = [\"general\"]\n",
            Refused(&["engines.w.type: required (line 2, column 1)"]),
        ),
        (
            "a duplicate key",
            "config_version = 1\nconfig_version = 1\n",
            Refused(&["config_version: defined more than once (line 2, column 1)"]),
        ),
        (
            "an unterminated string",
            "config_version = 1\n[search]\nregion = \"US\n",
            Refused(&[
                "TOML syntax error: invalid basic string, expected `\"` (line 3, column 13)",
            ]),
        ),
    ];
    assert!(cases.len() >= 36, "{} cases", cases.len());
    let mut wrong = Vec::new();
    for (name, text, expect) in cases {
        let found = problems(text);
        match (expect, found) {
            (Valid, None) => {}
            (Valid, Some(p)) => wrong.push(format!("{name}: refused: {p:?}")),
            (Refused(_), None) => wrong.push(format!("{name}: accepted")),
            (Refused(texts), Some(p)) => {
                let joined = p.join("\n");
                for wanted in *texts {
                    if !joined.contains(wanted) {
                        wrong.push(format!("{name}: no `{wanted}` in:\n    {joined}"));
                    }
                }
                for message in &p {
                    if let Some(word) = mentions_a_rust_type(message) {
                        wrong.push(format!("{name}: `{word}` in `{message}`"));
                    }
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// REQ-CFG-005 (RFC 020, DAT-12): where the range of a key depends on
/// another key, the message names the other key and its current value. The
/// seven cross-field cases of audit 020c, and the other pairs found by
/// reading `validate`.
#[test]
fn req_cfg_005_a_range_that_depends_on_another_key_says_so() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "deadline at its lowest",
            "config_version = 1\n[search]\ndeadline_ms = 500\n",
            "search.engine_timeout_ms: must be between 100 and 300 (search.deadline_ms 500 minus search.reserve_ms 200)",
        ),
        (
            "reserve at its highest",
            "config_version = 1\n[search]\nreserve_ms = 1000\n",
            "search.engine_timeout_ms: must be between 100 and 1500 (search.deadline_ms 2500 minus search.reserve_ms 1000)",
        ),
        (
            "nothing fits",
            "config_version = 1\n[search]\ndeadline_ms = 500\nreserve_ms = 1000\n",
            "search.engine_timeout_ms: no value fits: search.deadline_ms 500 minus search.reserve_ms 1000 leaves 0 ms, less than the minimum 100; raise search.deadline_ms or lower search.reserve_ms",
        ),
        (
            "max_results above its ceiling",
            "config_version = 1\n[search]\nmax_results_ceiling = 1\n",
            "search.max_results: must be between 1 and 1 (search.max_results_ceiling is 1)",
        ),
        (
            "max_chars above its ceiling",
            "config_version = 1\n[fetch]\nmax_chars_ceiling = 500\n",
            "fetch.max_chars: must be between 500 and 500 (fetch.max_chars_ceiling is 500)",
        ),
        (
            "fetch deadline at its highest",
            "config_version = 1\n[fetch]\ndeadline_ms = 30000\n",
            "service.http.request_timeout_s: must be at least 32 s (fetch.deadline_ms 30000 plus 2000 ms = 32000 ms)",
        ),
        (
            "request timeout at its lowest",
            "config_version = 1\n[service.http]\nrequest_timeout_s = 3\n",
            "service.http.request_timeout_s: must be at least 12 s (fetch.deadline_ms 10000 plus 2000 ms = 12000 ms)",
        ),
        (
            "max cooldown at its lowest",
            "config_version = 1\n[resilience]\nmax_cooldown_s = 10\n",
            "resilience.first_cooldown_s: must be between 10 and 10 (resilience.max_cooldown_s is 10)",
        ),
        (
            "a port that needs a rule",
            "config_version = 1\n[fetch]\nports = [80, 443, 8080]\n",
            "fetch.ports: 8080 needs an egress.allow rule",
        ),
    ];
    let mut wrong = Vec::new();
    for (name, text, wanted) in cases {
        match problems(text) {
            Some(p) if p.iter().any(|m| m == wanted) => {}
            other => wrong.push(format!("{name}: wanted `{wanted}`, got {other:?}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// REQ-CFG-005 (RFC 020, DAT-12): the valid edges of the dependent ranges
/// are still accepted (the change is in the wording, not in the rule).
#[test]
fn req_cfg_005_the_dependent_ranges_still_accept_their_edges() {
    for text in [
        "config_version = 1\n[search]\ndeadline_ms = 500\nengine_timeout_ms = 300\n",
        "config_version = 1\n[search]\ndeadline_ms = 2500\nreserve_ms = 1000\nengine_timeout_ms = 1500\n",
        "config_version = 1\n[search]\nmax_results_ceiling = 3\nmax_results = 3\n",
        "config_version = 1\n[fetch]\nmax_chars_ceiling = 500\nmax_chars = 500\n",
        "config_version = 1\n[fetch]\ndeadline_ms = 30000\n[service.http]\nrequest_timeout_s = 32\n",
        "config_version = 1\n[resilience]\nfirst_cooldown_s = 10\nmax_cooldown_s = 10\n",
    ] {
        assert_eq!(problems(text), None, "{text}");
    }
}

use proptest::prelude::*;

proptest! {
    /// REQ-CFG-005 (RFC 020, DAT-11): whatever the text, explaining its
    /// error does not panic (offsets fall on character boundaries, a
    /// multibyte character before the error moves nothing), and a message
    /// never contains a Rust type.
    #[test]
    fn req_cfg_005_explaining_any_text_never_panics(
        text in "(\\PC|\\n|\\r|\u{FEFF}|é|日){0,120}",
    ) {
        if let Err(error) = Config::parse(&text) {
            for message in &error.problems {
                prop_assert!(mentions_a_rust_type(message).is_none(), "{message}");
            }
        }
    }

    /// The same over documents that are nearly right: a valid file with one
    /// value replaced by a value of another kind.
    #[test]
    fn req_cfg_005_a_wrong_kind_of_value_names_its_key(
        key in prop::sample::select(vec![
            ("search", "max_results"), ("search", "deadline_ms"), ("fetch", "ports"),
            ("fetch", "robots_txt"), ("service.http", "max_sessions"), ("cache", "enabled"),
            ("privacy", "secret_detection"), ("logging", "level"), ("egress", "allow"),
        ]),
        value in prop::sample::select(vec!["\"x\"", "-1", "2.5", "true", "[1]", "{ a = 1 }", "99999999999999999999"]),
    ) {
        let text = format!("config_version = 1\n[{}]\n{} = {}\n", key.0, key.1, value);
        if let Err(error) = Config::parse(&text) {
            let wanted = format!("{}.{}", key.0, key.1);
            for message in &error.problems {
                prop_assert!(message.starts_with(&wanted), "{message}");
                prop_assert!(mentions_a_rust_type(message).is_none(), "{message}");
                prop_assert!(message.contains("(line 3, column "), "{message}");
            }
        }
    }
}

/// A string shaped like a key that someone pasted where a name belongs,
/// built at run time (a literal would look like a credential to a scanner).
fn pasted_key() -> String {
    ["BSAx9f", "K2mQ7v", "Lp3ZtR", "8wYc"].concat()
}

/// REQ-SEC-020 and REQ-CFG-005 (RFC 020, L1 of the 020l review): a value
/// from the file is never repeated in a message. An unknown word for an
/// enumeration is "an unknown word"; the list of allowed words is what the
/// reader needs.
#[test]
fn req_sec_020_an_unknown_word_is_not_repeated() {
    let word = ["hunter2", "-secret", "-value"].concat();
    for (name, text, wanted) in [
        (
            "secret_detection",
            format!("config_version = 1\n[privacy]\nsecret_detection = \"{word}\"\n"),
            "privacy.secret_detection: expected one of off, warn, mask, refuse, found an unknown word (line 3, column 20)",
        ),
        (
            "a category",
            format!(
                "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"{word}\"]\n"
            ),
            "engines.w.categories[0]: expected one of general, news, science, code, found an unknown word (line 4, column 15)",
        ),
        (
            "the log level",
            format!("config_version = 1\n[logging]\nlevel = \"{word}\"\n"),
            "logging.level: expected one of error, warn, info, debug, found an unknown word (line 3, column 9)",
        ),
    ] {
        let found = problems(&text).unwrap_or_else(|| panic!("{name}: accepted"));
        assert_eq!(found, vec![wanted.to_owned()], "{name}");
        assert!(!found.join("\n").contains("hunter2"), "{name}");
    }
}

/// REQ-SEC-020 (RFC 020, L1 of the 020l review): `api_key_env` and
/// `token_env` name a variable that holds the secret. The most common
/// mistake is to paste the secret into them, and the message must not bring
/// it back: a name that does not look like a variable name (upper case
/// letters, digits and underscores, at most 64) is not shown. A proper name
/// is still named; a valid lower-case name is accepted and is not echoed
/// when its variable is unset.
#[test]
fn req_sec_020_a_pasted_secret_in_a_variable_name_is_not_repeated() {
    use crate::config::{SecretError, SecretSource};
    let pasted = pasted_key();
    let not_a_name = "the value is not the name of an environment variable (it is not shown); this key names a variable that holds the secret, such as BRAVE_API_KEY, it does not hold the secret";
    for (name, value) in [
        ("a pasted key", pasted.clone()),
        ("lower case", "my_brave_key".to_owned()),
        ("a dash", "BRAVE-KEY".to_owned()),
        ("longer than 64", "A".repeat(65)),
        ("empty", String::new()),
    ] {
        let error = SecretSource::Env(value.clone()).resolve();
        // An empty or unset variable of such a name does not exist in the test environment.
        let message = error.unwrap_err().to_string();
        assert_eq!(message, not_a_name, "{name}");
        if !value.is_empty() {
            assert!(!message.contains(&value), "{name}");
        }
    }
    for value in ["RYPSOK_TEST_UNSET_7F3A", "_X", "A", &"A".repeat(64)] {
        let message = SecretError::EnvUnset(value.to_owned()).to_string();
        assert_eq!(
            message,
            format!("environment variable {value} is unset or empty")
        );
    }
    // In a configuration: valid, and silent about the value when unset.
    let text = format!(
        "config_version = 1\n[engines.b]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"{pasted}\"\n"
    );
    let config = Config::parse(&text).unwrap();
    assert!(config.validate().is_ok(), "{:?}", config.validate());
    let table = &config.engines["b"];
    let error = table.secret_source().unwrap().resolve().unwrap_err();
    assert!(!error.to_string().contains(&pasted), "{error}");
}
