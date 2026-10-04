//! The content boundary: every piece of external text passes here before it
//! enters a response. Cleaning, bounding, removal of invisible characters,
//! notices; never a rewrite of visible text (external design §9; RFC 005).
//! Filled in milestone M1.

#[cfg(test)]
mod tests;
