//! Bounded protocol detection (G1 of the 020g review).
//!
//! `hyper_util`'s `auto::Builder` tells HTTP/1 from HTTP/2 by reading up to
//! the 24-byte HTTP/2 preface, and it starts no timer while it does so. A
//! peer that sends `P` and stops therefore holds its connection (and its
//! LIM-H1 permit) for as long as it likes. This module does the detection
//! itself, under one deadline set by the caller, and hands the bytes it read
//! back to hyper in front of the stream.
//!
//! Read-and-replay rather than a `peek` loop: a peek returns at once while
//! data is buffered, so waiting for the next byte would need a sleep (a poll
//! per stalled connection). Reading waits on the socket and costs nothing
//! while the peer is silent.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

/// The HTTP/2 client connection preface (RFC 9113, section 3.4).
const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// True when the bytes read so far decide the protocol: the first bytes
/// differ from the preface (HTTP/1), or the whole preface has arrived
/// (HTTP/2). `P` and `PR` decide nothing.
pub(super) fn decided(read: &[u8]) -> bool {
    read.len() >= H2_PREFACE.len() || !H2_PREFACE.starts_with(read)
}

/// Reads (consumes) from `stream` until [`decided`] holds or the peer
/// closes. Returns the bytes read, to be replayed. It has no deadline of
/// its own: the caller wraps the whole call in one `timeout`, so a trickle
/// of bytes does not restart anything.
pub(super) async fn read_until_decided<S>(stream: &mut S) -> io::Result<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut read = Vec::with_capacity(H2_PREFACE.len());
    let mut chunk = [0u8; 24];
    while !decided(&read) {
        let wanted = H2_PREFACE.len() - read.len();
        let n = stream.read(&mut chunk[..wanted]).await?;
        if n == 0 {
            break;
        }
        read.extend_from_slice(&chunk[..n]);
    }
    Ok(read)
}

/// A stream that first yields `prefix`, then reads from `inner`. Writes go
/// straight to `inner`.
pub(super) struct Replay<S> {
    prefix: Vec<u8>,
    position: usize,
    inner: S,
}

impl<S> Replay<S> {
    pub(super) fn new(prefix: Vec<u8>, inner: S) -> Self {
        Self {
            prefix,
            position: 0,
            inner,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Replay<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        if this.position < this.prefix.len() {
            let left = &this.prefix[this.position..];
            let n = left.len().min(buf.remaining());
            buf.put_slice(&left[..n]);
            this.position += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Replay<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}

#[cfg(test)]
mod tests;
