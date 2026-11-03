//! Tests of deduplication and rank fusion. Every test names the
//! requirement it verifies.

use super::*;
use crate::search::normalize::{comparison_key, returned_url};

fn observe(
    engine_index: usize,
    engine: &str,
    preference: u32,
    rank: u32,
    url: &str,
) -> Observation {
    let returned_url = returned_url(url, 2048).unwrap();
    Observation {
        engine_index,
        engine: EngineId::new(engine).unwrap(),
        preference,
        rank,
        raw: RawResult::new(format!("{engine} {rank}"), url, ""),
        returned_url: returned_url.clone(),
        key: comparison_key(&returned_url),
    }
}

/// REQ-RANK-004: observations with one key merge; the best rank wins the
/// fields, ties break by preference; engines are listed in selection order.
#[test]
fn req_rank_004_merge_keeps_best_observation_and_engines() {
    let merged = merge(vec![
        observe(0, "a", 10, 2, "https://example.org/x"),
        observe(1, "b", 20, 1, "https://www.example.org/x/"),
        observe(0, "a", 10, 1, "https://example.org/y"),
        observe(2, "c", 5, 1, "http://example.org/x?utm_source=z"),
    ]);
    assert_eq!(merged.len(), 2);
    let x = &merged[0];
    assert_eq!(
        x.best.raw.title, "c 1",
        "best rank 1 with lowest preference wins"
    );
    assert_eq!(
        x.engines.iter().map(|e| e.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(merged[1].best.raw.title, "a 1");
}

/// REQ-RANK-005: reciprocal rank fusion in integers; a result two engines
/// returned at rank 3 outranks one engine's rank 1.
#[test]
fn req_rank_005_fusion_scores_are_integer_and_reward_agreement() {
    let merged = merge(vec![
        observe(0, "a", 10, 1, "https://example.org/solo"),
        observe(0, "a", 10, 3, "https://example.org/agreed"),
        observe(1, "b", 20, 3, "https://example.org/agreed"),
    ]);
    let ordered = order(merged);
    // 1_000_000 / 63 = 15_873 per engine at rank 3, twice: 31_746; rank 1 alone: 16_393.
    assert_eq!(ordered[0].best.returned_url, "https://example.org/agreed");
    assert_eq!(ordered[0].score, 2 * (RRF_SCALE / (RRF_K + 3)));
    assert_eq!(ordered[0].score, 31_746);
    assert_eq!(ordered[1].score, RRF_SCALE / (RRF_K + 1));
    assert_eq!(ordered[1].score, 16_393);
}

/// REQ-RANK-006: ties resolve by best rank, preference, then key.
#[test]
fn req_rank_006_total_order() {
    let merged = merge(vec![
        observe(0, "a", 10, 1, "https://example.org/b"),
        observe(1, "b", 5, 1, "https://example.org/a"),
        observe(0, "a", 10, 2, "https://example.org/c"),
        observe(1, "b", 5, 2, "https://example.org/c"),
    ]);
    let ordered = order(merged);
    let urls: Vec<&str> = ordered
        .iter()
        .map(|m| m.best.returned_url.as_str())
        .collect();
    assert_eq!(
        urls,
        vec![
            "https://example.org/c",
            "https://example.org/a",
            "https://example.org/b"
        ]
    );
}

/// REQ-ARCH-004: the same observations in another arrival order yield the
/// same ordered output when the selection order is fixed.
#[test]
fn req_arch_004_fusion_is_independent_of_arrival_order() {
    let a = vec![
        observe(0, "a", 10, 1, "https://example.org/1"),
        observe(0, "a", 10, 2, "https://example.org/2"),
        observe(1, "b", 20, 1, "https://example.org/2"),
        observe(1, "b", 20, 2, "https://example.org/3"),
    ];
    let mut b = a.clone();
    b.reverse();
    let first = order(merge(a));
    let second = order(merge(b));
    let urls = |m: &[Merged]| {
        m.iter()
            .map(|x| (x.best.returned_url.clone(), x.score, x.engines.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(urls(&first), urls(&second));
}

/// REQ-RANK-001: a merged result holds the returned URL, the comparison
/// key, title, snippet, date, and every observation with its rank.
#[test]
fn req_rank_001_merged_result_holds_observations_with_ranks() {
    let merged = merge(vec![
        observe(0, "a", 10, 2, "https://example.org/x"),
        observe(1, "b", 20, 1, "https://example.org/x"),
    ]);
    let x = &merged[0];
    assert_eq!(x.best.returned_url, "https://example.org/x");
    assert_eq!(x.best.key, comparison_key("https://example.org/x"));
    assert_eq!(
        x.ranks,
        vec![
            (EngineId::new("a").unwrap(), 2),
            (EngineId::new("b").unwrap(), 1)
        ]
    );
    assert_eq!(x.best.raw.title, "b 1");
}

/// REQ-RANK-004 (RFC 020 D-8): when `http` and `https` observations merge
/// under one key, the user is shown the `https` URL even if the `http` one
/// ranked first; the fusion and the other fields do not change. The URL is
/// always one engine's own, byte for byte.
#[test]
fn req_rank_004_the_https_url_is_shown_when_the_best_is_http() {
    let merged = merge(vec![
        observe(0, "a", 10, 1, "http://x.example/p"),
        observe(1, "b", 20, 3, "https://x.example/p"),
    ]);
    assert_eq!(merged.len(), 1);
    let m = &merged[0];
    assert_eq!(
        m.best.returned_url, "http://x.example/p",
        "rank 1 stays the best"
    );
    assert_eq!(m.best.raw.title, "a 1", "title and snippet are the best's");
    assert_eq!(m.url, "https://x.example/p", "the https URL is shown");
    assert_eq!(m.score, RRF_SCALE / 61 + RRF_SCALE / 63, "fusion unchanged");
    assert_eq!(m.ranks.len(), 2);
}

/// REQ-RANK-004 (RFC 020 D-8): which URL is shown, case by case.
#[test]
fn req_rank_004_the_shown_url_follows_the_rule() {
    let shown = |obs: Vec<Observation>| merge(obs).remove(0).url;
    // The best is https: it is shown, whatever else merged.
    assert_eq!(
        shown(vec![
            observe(0, "a", 10, 1, "https://x.example/p"),
            observe(1, "b", 20, 2, "http://x.example/p"),
        ]),
        "https://x.example/p"
    );
    // Only http observations: the best-ranked one.
    assert_eq!(
        shown(vec![
            observe(0, "a", 10, 2, "http://x.example/p?a=1"),
            observe(1, "b", 20, 1, "http://www.x.example/p?a=1"),
        ]),
        "http://www.x.example/p?a=1"
    );
    // Two https observations under an http best: the best-ranked https one,
    // with its own bytes (the www form here), by rank, then preference.
    assert_eq!(
        shown(vec![
            observe(0, "a", 10, 1, "http://x.example/p"),
            observe(1, "b", 30, 2, "https://x.example/p"),
            observe(2, "c", 20, 2, "https://www.x.example/p"),
        ]),
        "https://www.x.example/p"
    );
    // Independent of the arrival order of the observations (REQ-ARCH-004).
    let forward = vec![
        observe(0, "a", 10, 1, "http://x.example/p"),
        observe(1, "b", 30, 2, "https://x.example/p"),
        observe(2, "c", 20, 2, "https://www.x.example/p"),
    ];
    let mut backward = forward.clone();
    backward.reverse();
    assert_eq!(shown(forward), shown(backward));
}
