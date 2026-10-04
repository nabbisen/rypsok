//! The search pipeline: validation, privacy check, engine selection,
//! fan-out under one deadline, collection, normalization, deduplication,
//! rank fusion, the content boundary and the response (external design §6
//! and §7; RFC 007 and RFC 008). Filled in milestone M1.

#[cfg(test)]
mod tests;
