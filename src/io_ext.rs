use std::{
    pin::Pin,
    task::{self, Poll},
};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};

pin_project! {
    pub struct Unsplit<A, B> {
        #[pin]
        reader: A,
        #[pin]
        writer: B,
    }
}

impl<A, B> Unsplit<A, B> {
    pub fn new(reader: A, writer: B) -> Self {
        Self { reader, writer }
    }
}

impl<A: AsyncRead, B: AsyncWrite> AsyncWrite for Unsplit<A, B> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        self.project().writer.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<tokio::io::Result<()>> {
        self.project().writer.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        self.project().writer.poll_shutdown(cx)
    }
}

impl<A: AsyncRead, B: AsyncWrite> AsyncRead for Unsplit<A, B> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        self.project().reader.poll_read(cx, buf)
    }
}
