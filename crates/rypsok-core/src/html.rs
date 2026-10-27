//! HTML shared by the fetch pipeline and the scraping drivers (RFC 012,
//! RFC 015): charset decoding and the bounded DOM. Pure, synchronous,
//! bounded by LIM-F10 nodes; callers run it off the asynchronous runtime
//! under LIM-C5.

pub mod charset;
pub mod dom;
