//! The limits table as typed constants, mirrored from
//! `docs/src/maintainers/limits-and-defaults.md` and compared with it by
//! `tools/check-limits` (RFC 004).
//!
//! Every numeric row of the table is one constant in [`defaults`], annotated
//! with its `LIM-` identifier in a trailing comment that the script reads.
//! [`Limits`] carries the effective values after configuration; its
//! [`Limits::DEFAULT`] is the table.

/// The defaults of the limits table. Units: characters are Unicode scalar
/// values (LIM-U1), bytes are after decompression (LIM-U2), time is
/// milliseconds unless the name says seconds (LIM-U3).
pub mod defaults {
    /// Length of `query`.
    pub const QUERY_MAX_CHARS: u32 = 400; // LIM-S1
    /// Default of `max_results`.
    pub const MAX_RESULTS: u8 = 5; // LIM-S2
    /// Upper bound of `max_results`.
    pub const MAX_RESULTS_CEILING: u8 = 20;
    /// Results requested from each engine.
    pub const RESULTS_PER_ENGINE: u8 = 10; // LIM-S3
    /// Overall deadline of a search.
    pub const SEARCH_DEADLINE_MS: u32 = 2500; // LIM-S4
    /// Timeout of one engine request.
    pub const ENGINE_TIMEOUT_MS: u32 = 2000; // LIM-S5
    /// Reserve for post-processing.
    pub const SEARCH_RESERVE_MS: u32 = 200; // LIM-S6
    /// Margin of a `web_search` call over the search deadline (LIM-S7 is
    /// LIM-S4 plus this margin).
    pub const SEARCH_CALL_MARGIN_MS: u32 = 300;
    /// Engines queried per request.
    pub const ENGINES_PER_REQUEST: u8 = 8; // LIM-S8
    /// Length of a snippet.
    pub const SNIPPET_MAX_CHARS: u32 = 300; // LIM-S9
    /// Length of a title.
    pub const TITLE_MAX_CHARS: u32 = 150; // LIM-S10
    /// Window for the typical latency of an engine, in completed requests.
    pub const LATENCY_WINDOW: u8 = 20; // LIM-S11
    /// Connect timeout of an engine request.
    pub const ENGINE_CONNECT_TIMEOUT_MS: u32 = 1000; // LIM-S12
    /// Maximum duration of an `engine_status` call.
    pub const STATUS_CALL_MAX_MS: u32 = 200; // LIM-S13

    /// Default of `max_chars`.
    pub const FETCH_MAX_CHARS: u32 = 3000; // LIM-F1
    /// Ceiling of `max_chars`.
    pub const FETCH_MAX_CHARS_CEILING: u32 = 20000; // LIM-F2
    /// Response bytes after decompression.
    pub const FETCH_MAX_BYTES: u64 = 5_242_880; // LIM-F3
    /// Overall deadline of a fetch.
    pub const FETCH_DEADLINE_MS: u32 = 10000; // LIM-F4
    /// Margin of a `web_fetch` call over the fetch deadline (LIM-F5 is
    /// LIM-F4 plus this margin).
    pub const FETCH_CALL_MARGIN_MS: u32 = 1000;
    /// Connect timeout of a fetch.
    pub const FETCH_CONNECT_TIMEOUT_MS: u32 = 3000; // LIM-F6
    /// Redirects followed on the same host.
    pub const FETCH_MAX_REDIRECTS: u8 = 5; // LIM-F7
    /// Ports a fetch may use (LIM-F8).
    pub const FETCH_PORTS: &[u16] = &[80, 443];
    /// Length of `url`.
    pub const URL_MAX_CHARS: u32 = 2048; // LIM-F9
    /// HTML nodes parsed.
    pub const HTML_MAX_NODES: u32 = 100000; // LIM-F10
    /// HTML nesting depth.
    pub const HTML_MAX_DEPTH: u32 = 256; // LIM-F11
    /// Seconds a robots.txt is kept in memory.
    pub const ROBOTS_TTL_S: u32 = 86400; // LIM-F12
    /// Size of a robots.txt read, in bytes.
    pub const ROBOTS_MAX_BYTES: u64 = 524_288; // LIM-F13
    /// URLs remembered for strict mode (RFC 011; used from milestone M3).
    pub const STRICT_MODE_URLS: u32 = 10000; // LIM-F14
    /// robots.txt entries kept in memory (RFC 011; used from milestone M3).
    pub const ROBOTS_CACHE_ENTRIES: u32 = 1000; // LIM-F15

    /// Outbound requests in flight, whole process.
    pub const OUTBOUND_IN_FLIGHT: u32 = 32; // LIM-C1
    /// Requests in flight per engine.
    pub const PER_ENGINE_IN_FLIGHT: u8 = 2; // LIM-C2
    /// Fetches in flight, whole process.
    pub const FETCHES_IN_FLIGHT: u32 = 4; // LIM-C3
    /// Fetches in flight per host.
    pub const FETCHES_PER_HOST: u8 = 1; // LIM-C4
    /// Parsing tasks in flight.
    pub const PARSING_IN_FLIGHT: u32 = 2; // LIM-C5
    /// `web_search` calls accepted per minute.
    pub const WEB_SEARCH_PER_MINUTE: u32 = 60; // LIM-C6
    /// `web_fetch` calls accepted per minute.
    pub const WEB_FETCH_PER_MINUTE: u32 = 30; // LIM-C7
    /// `engine_status` calls accepted per minute.
    pub const ENGINE_STATUS_PER_MINUTE: u32 = 30; // LIM-C8

    /// Consecutive failures that open the circuit.
    pub const FAILURES_TO_OPEN: u8 = 3; // LIM-B1
    /// First cooldown, in seconds.
    pub const FIRST_COOLDOWN_S: u32 = 60; // LIM-B2
    /// Longest cooldown, in seconds.
    pub const MAX_COOLDOWN_S: u32 = 900; // LIM-B3

    /// Characters of all tool definitions and the server instructions.
    pub const DEFINITIONS_MAX_CHARS: u32 = 2000; // LIM-I1
    /// Parameters of one tool.
    pub const PARAMETERS_PER_TOOL: u8 = 5; // LIM-I2
    /// Length of a notice or error message.
    pub const MESSAGE_MAX_CHARS: u32 = 200; // LIM-I4

    /// Time to exit after end of file on stdin or `SIGTERM`.
    pub const SHUTDOWN_MAX_MS: u32 = 3000; // LIM-O1
    /// Time until a cancelled call has aborted its outbound requests.
    pub const CANCEL_MAX_MS: u32 = 500; // LIM-O2

    /// Lifetime of a cache entry, in seconds.
    pub const CACHE_TTL_S: u32 = 300; // LIM-K2
    /// Cache entries.
    pub const CACHE_ENTRIES: u32 = 1000; // LIM-K3
}

/// The effective limits of a process: the table's defaults, overridden by
/// configuration within the table's ranges (RFC 004). Field names follow
/// the constants of [`defaults`]; each doc comment cites the identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// LIM-S1.
    pub query_max_chars: u32,
    /// LIM-S2, default.
    pub max_results: u8,
    /// LIM-S2, upper bound.
    pub max_results_ceiling: u8,
    /// LIM-S3.
    pub results_per_engine: u8,
    /// LIM-S4.
    pub search_deadline_ms: u32,
    /// LIM-S5.
    pub engine_timeout_ms: u32,
    /// LIM-S6.
    pub search_reserve_ms: u32,
    /// LIM-S8.
    pub engines_per_request: u8,
    /// LIM-S9.
    pub snippet_max_chars: u32,
    /// LIM-S10.
    pub title_max_chars: u32,
    /// LIM-S11.
    pub latency_window: u8,
    /// LIM-S12.
    pub engine_connect_timeout_ms: u32,
    /// LIM-F1.
    pub fetch_max_chars: u32,
    /// LIM-F2.
    pub fetch_max_chars_ceiling: u32,
    /// LIM-F3.
    pub fetch_max_bytes: u64,
    /// LIM-F4.
    pub fetch_deadline_ms: u32,
    /// LIM-F6.
    pub fetch_connect_timeout_ms: u32,
    /// LIM-F7.
    pub fetch_max_redirects: u8,
    /// LIM-F8.
    pub fetch_ports: Vec<u16>,
    /// LIM-F9.
    pub url_max_chars: u32,
    /// LIM-C1.
    pub outbound_in_flight: u32,
    /// LIM-C2.
    pub per_engine_in_flight: u8,
    /// LIM-C3.
    pub fetches_in_flight: u32,
    /// LIM-C5.
    pub parsing_in_flight: u32,
    /// LIM-C6.
    pub web_search_per_minute: u32,
    /// LIM-C7.
    pub web_fetch_per_minute: u32,
    /// LIM-C8.
    pub engine_status_per_minute: u32,
    /// LIM-B1.
    pub failures_to_open: u8,
    /// LIM-B2.
    pub first_cooldown_s: u32,
    /// LIM-B3.
    pub max_cooldown_s: u32,
}

impl Limits {
    /// The limits table as documented.
    pub const DEFAULT: Limits = Limits {
        query_max_chars: defaults::QUERY_MAX_CHARS,
        max_results: defaults::MAX_RESULTS,
        max_results_ceiling: defaults::MAX_RESULTS_CEILING,
        results_per_engine: defaults::RESULTS_PER_ENGINE,
        search_deadline_ms: defaults::SEARCH_DEADLINE_MS,
        engine_timeout_ms: defaults::ENGINE_TIMEOUT_MS,
        search_reserve_ms: defaults::SEARCH_RESERVE_MS,
        engines_per_request: defaults::ENGINES_PER_REQUEST,
        snippet_max_chars: defaults::SNIPPET_MAX_CHARS,
        title_max_chars: defaults::TITLE_MAX_CHARS,
        latency_window: defaults::LATENCY_WINDOW,
        engine_connect_timeout_ms: defaults::ENGINE_CONNECT_TIMEOUT_MS,
        fetch_max_chars: defaults::FETCH_MAX_CHARS,
        fetch_max_chars_ceiling: defaults::FETCH_MAX_CHARS_CEILING,
        fetch_max_bytes: defaults::FETCH_MAX_BYTES,
        fetch_deadline_ms: defaults::FETCH_DEADLINE_MS,
        fetch_connect_timeout_ms: defaults::FETCH_CONNECT_TIMEOUT_MS,
        fetch_max_redirects: defaults::FETCH_MAX_REDIRECTS,
        fetch_ports: Vec::new(),
        url_max_chars: defaults::URL_MAX_CHARS,
        outbound_in_flight: defaults::OUTBOUND_IN_FLIGHT,
        per_engine_in_flight: defaults::PER_ENGINE_IN_FLIGHT,
        fetches_in_flight: defaults::FETCHES_IN_FLIGHT,
        parsing_in_flight: defaults::PARSING_IN_FLIGHT,
        web_search_per_minute: defaults::WEB_SEARCH_PER_MINUTE,
        web_fetch_per_minute: defaults::WEB_FETCH_PER_MINUTE,
        engine_status_per_minute: defaults::ENGINE_STATUS_PER_MINUTE,
        failures_to_open: defaults::FAILURES_TO_OPEN,
        first_cooldown_s: defaults::FIRST_COOLDOWN_S,
        max_cooldown_s: defaults::MAX_COOLDOWN_S,
    };

    /// Maximum duration of a `web_search` call (LIM-S7).
    pub fn search_call_max_ms(&self) -> u32 {
        self.search_deadline_ms + defaults::SEARCH_CALL_MARGIN_MS
    }

    /// Maximum duration of a `web_fetch` call (LIM-F5).
    pub fn fetch_call_max_ms(&self) -> u32 {
        self.fetch_deadline_ms + defaults::FETCH_CALL_MARGIN_MS
    }
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            fetch_ports: defaults::FETCH_PORTS.to_vec(),
            ..Limits::DEFAULT
        }
    }
}

#[cfg(test)]
mod tests;
