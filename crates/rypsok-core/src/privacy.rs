//! The secret check on queries and URLs, refusing by default and naming the
//! kind and position of a finding, never its value (external design §10;
//! RFC 009). Filled in milestone M1.

#[cfg(test)]
mod tests;
