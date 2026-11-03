use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

/// REQ-SEC-022 (G1): what decides the protocol.
#[test]
fn req_sec_022_the_preface_rule_decides_only_on_a_mismatch_or_the_whole_preface() {
    assert!(!decided(b""), "nothing read decides nothing");
    assert!(!decided(b"P"), "P starts both POST and PRI");
    assert!(!decided(b"PRI * HTTP/2.0\r\n\r\nSM\r\n"), "21 of 24 bytes");
    assert!(decided(b"PO"), "O differs from R: HTTP/1");
    assert!(decided(b"GET / HTTP/1.1\r\n"), "an ordinary request");
    assert!(decided(H2_PREFACE), "the whole preface: HTTP/2");
    assert!(
        decided(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\nX"),
        "more than the preface"
    );
}

/// REQ-SEC-022 (G1): detection consumes only what it needs, and the replay
/// gives those bytes back first, then the rest of the stream, unchanged.
#[tokio::test]
async fn req_sec_022_the_replayed_stream_is_the_original_byte_for_byte() {
    let (mut client, mut server) = tokio::io::duplex(1024);
    let request = b"POST /mcp HTTP/1.1\r\nHost: x\r\n\r\nbody".to_vec();
    client.write_all(&request).await.unwrap();
    drop(client);
    let prefix = read_until_decided(&mut server).await.unwrap();
    assert!(prefix.len() <= 24 && decided(&prefix));
    let mut replay = Replay::new(prefix, server);
    let mut all = Vec::new();
    replay.read_to_end(&mut all).await.unwrap();
    assert_eq!(all, request);
}

/// REQ-SEC-022 (G1): a peer that closes before the protocol is decided ends
/// detection without an error; the replay carries what it sent.
#[tokio::test]
async fn req_sec_022_a_close_before_the_decision_returns_what_was_read() {
    let (mut client, mut server) = tokio::io::duplex(64);
    client.write_all(b"P").await.unwrap();
    drop(client);
    assert_eq!(read_until_decided(&mut server).await.unwrap(), b"P");
}

/// REQ-SEC-022 (G1): writes pass through to the stream.
#[tokio::test]
async fn req_sec_022_writes_pass_through_the_replay() {
    let (mut client, server) = tokio::io::duplex(64);
    let mut replay = Replay::new(b"x".to_vec(), server);
    replay.write_all(b"answer").await.unwrap();
    replay.flush().await.unwrap();
    let mut got = [0u8; 6];
    client.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, b"answer");
}
