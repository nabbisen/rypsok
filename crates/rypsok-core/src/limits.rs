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

    /// HTTP connections accepted at once (RFC 013).
    pub const HTTP_MAX_CONNECTIONS: u32 = 64; // LIM-H1
    /// Size of an HTTP request body, in bytes (RFC 013).
    pub const HTTP_MAX_REQUEST_BYTES: u64 = 65536; // LIM-H2
    /// Idle HTTP connection timeout, in seconds (RFC 013).
    pub const HTTP_IDLE_TIMEOUT_S: u32 = 60; // LIM-H3
    /// Streams one HTTP/2 connection may open (RFC 018).
    pub const HTTP_MAX_STREAMS: u32 = 16; // LIM-H4
    /// Time one HTTP request may take, whatever its state, in seconds (RFC 018).
    pub const HTTP_REQUEST_TIMEOUT_S: u32 = 30; // LIM-H5
    /// Live sessions of the handshake-based revisions over HTTP (RFC 019).
    pub const HTTP_MAX_SESSIONS: u32 = 256; // LIM-H6
    /// Size of the header block of an HTTP request, in bytes: the request
    /// line and headers of HTTP/1, the header list of HTTP/2 (fixed, RFC 020).
    pub const HTTP_MAX_HEADER_BYTES: u32 = 16384; // LIM-H7

    /// Lifetime of a cache entry, in seconds.
    pub const CACHE_TTL_S: u32 = 300; // LIM-K2
    /// Cache entries.
    pub const CACHE_ENTRIES: u32 = 1000; // LIM-K3
    /// Size of one cache entry, in bytes (RFC 014).
    pub const CACHE_ENTRY_BYTES: u32 = 32768; // LIM-K4

    // The measured costs of the memory estimate (RFC 020 D-20, D-21). They
    // come from audit 020e, taken on the delivered musl release build of
    // 0.8.0, and are rounded up: the estimate is on the safe side.

    /// The process at rest: 10 to 12 MiB idle (median 10.9) and a margin.
    pub const MEASURED_BASE_BYTES: u64 = 13_631_488; // LIM-M1
    /// The part of one parse that does not grow with the page: the line of
    /// 34.1 MiB (5 MiB page) and 99.2 MiB (20 MiB page) crosses the
    /// vertical axis at 12.4 MiB.
    pub const MEASURED_PARSE_FIXED_BYTES: u64 = 13_631_488; // LIM-M2
    /// The part of one parse that grows with the page, in tenths of a byte
    /// per byte: the same line rises 4.34 per byte, rounded up to 4.5.
    pub const MEASURED_PARSE_TENTHS_PER_BYTE: u64 = 45; // LIM-M3
    /// One live session of the handshake-based revisions: 58.5 KiB
    /// measured (56.7 to 60.6), rounded up.
    pub const MEASURED_SESSION_BYTES: u64 = 61_440; // LIM-M4
    /// `MemoryMax` of the documented systemd unit, the figure the estimate
    /// is compared with.
    pub const MEMORY_UNIT_BYTES: u64 = 268_435_456; // LIM-M5
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
    /// LIM-F10.
    pub html_max_nodes: u32,
    /// LIM-F11.
    pub html_max_depth: u32,
    /// LIM-F12.
    pub robots_ttl_s: u32,
    /// LIM-F13.
    pub robots_max_bytes: u64,
    /// LIM-F14.
    pub strict_mode_urls: u32,
    /// LIM-F15.
    pub robots_cache_entries: u32,
    /// LIM-H1.
    pub http_max_connections: u32,
    /// LIM-H2.
    pub http_max_request_bytes: u64,
    /// LIM-H3.
    pub http_idle_timeout_s: u32,
    /// LIM-H4.
    pub http_max_streams: u32,
    /// LIM-H5.
    pub http_request_timeout_s: u32,
    /// LIM-H6.
    pub http_max_sessions: u32,
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

/// The worst-case memory of a process, from its effective limits and the
/// measured costs of audit 020e (RFC 020 D-20, D-21): an estimate, measured
/// on the musl release build, not a guarantee. Bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MemoryEstimate {
    /// The process at rest (LIM-M1).
    pub base: u64,
    /// The bodies being read: fetches in flight times `fetch.max_bytes`.
    pub bodies: u64,
    /// The parses: parsing tasks in flight times the cost of one.
    pub parses: u64,
    /// The sessions of the handshake-based revisions.
    pub sessions: u64,
    /// The result cache bound, when it is on.
    pub cache: u64,
}

impl MemoryEstimate {
    /// The sum, in bytes.
    pub fn total_bytes(&self) -> u64 {
        self.base + self.bodies + self.parses + self.sessions + self.cache
    }

    /// A number of bytes in whole MiB, rounded up.
    pub fn mib(bytes: u64) -> u64 {
        bytes.div_ceil(1024 * 1024)
    }

    /// The sum in whole MiB, rounded up.
    pub fn total_mib(&self) -> u64 {
        Self::mib(self.total_bytes())
    }

    /// Whether the sum is above `MemoryMax` of the documented unit.
    pub fn exceeds_unit(&self) -> bool {
        self.total_bytes() > defaults::MEMORY_UNIT_BYTES
    }
}

impl Limits {
    /// The worst-case memory: `13 MiB + fetches x max_bytes + parses x
    /// (4.5 x max_bytes + 13 MiB) + sessions x 60 KiB`, and `cache_bytes`
    /// (LIM-K3 times LIM-K4 when the cache is on, else 0).
    pub fn worst_case_memory(&self, cache_bytes: u64) -> MemoryEstimate {
        let page = self.fetch_max_bytes;
        let one_parse = page * defaults::MEASURED_PARSE_TENTHS_PER_BYTE / 10
            + defaults::MEASURED_PARSE_FIXED_BYTES;
        MemoryEstimate {
            base: defaults::MEASURED_BASE_BYTES,
            bodies: u64::from(self.fetches_in_flight) * page,
            parses: u64::from(self.parsing_in_flight) * one_parse,
            sessions: u64::from(self.http_max_sessions) * defaults::MEASURED_SESSION_BYTES,
            cache: cache_bytes,
        }
    }

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
        html_max_nodes: defaults::HTML_MAX_NODES,
        html_max_depth: defaults::HTML_MAX_DEPTH,
        robots_ttl_s: defaults::ROBOTS_TTL_S,
        robots_max_bytes: defaults::ROBOTS_MAX_BYTES,
        strict_mode_urls: defaults::STRICT_MODE_URLS,
        robots_cache_entries: defaults::ROBOTS_CACHE_ENTRIES,
        http_max_connections: defaults::HTTP_MAX_CONNECTIONS,
        http_max_request_bytes: defaults::HTTP_MAX_REQUEST_BYTES,
        http_idle_timeout_s: defaults::HTTP_IDLE_TIMEOUT_S,
        http_max_streams: defaults::HTTP_MAX_STREAMS,
        http_request_timeout_s: defaults::HTTP_REQUEST_TIMEOUT_S,
        http_max_sessions: defaults::HTTP_MAX_SESSIONS,
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
