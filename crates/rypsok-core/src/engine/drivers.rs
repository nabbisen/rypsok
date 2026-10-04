//! One module per driver, each behind a Cargo feature: `wikipedia` under
//! `engine-wikipedia`, `brave` under `engine-brave`. Both arrive in
//! milestone M1 with their admission records and fixtures (REQ-CMP-001,
//! REQ-TEST-002).

#[cfg(test)]
mod tests;
