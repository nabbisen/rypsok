//! The fetch pipeline: egress policy, connect-time enforcement, redirects,
//! robots.txt, bounded download, extraction and continuation (external
//! design §3.2; RFC 012). Arrives in milestone M3; empty until then.

#[cfg(test)]
mod tests;
