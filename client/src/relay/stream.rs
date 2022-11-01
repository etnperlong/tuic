use futures_util::{Future, Stream};
use quinn::{
    Connection as QuinnConnection, ConnectionError, RecvStream as QuinnRecvStream,
    SendStream as QuinnSendStream,
};
use std::{
    io::{Error, IoSlice, Result},
    pin::Pin,
    result::Result as StdResult,
    sync::{Arc, Weak},
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct SendStream {
    send: QuinnSendStream,
    _reg: Register,
}

impl SendStream {
    #[inline]
    pub fn new(send: QuinnSendStream, reg: Register) -> Self {
        Self { send, _reg: reg }
    }

    #[inline]
    pub async fn finish(&mut self) -> Result<()> {
        self.send.finish().await.map_err(Error::from)
    }
}

impl AsyncWrite for SendStream {
    #[inline]
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.send).poll_write(cx, buf)
    }

    #[inline]
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.send).poll_write_vectored(cx, bufs)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.send.is_write_vectored()
    }

    #[inline]
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}

pub struct RecvStream {
    recv: QuinnRecvStream,
    _reg: Register,
}

impl RecvStream {
    #[inline]
    pub fn new(recv: QuinnRecvStream, reg: Register) -> Self {
        Self { recv, _reg: reg }
    }
}

impl AsyncRead for RecvStream {
    #[inline]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

pub struct BiStream {
    send: SendStream,
    recv: RecvStream,
}

impl BiStream {
    #[inline]
    pub fn new(send: SendStream, recv: RecvStream) -> Self {
        Self { send, recv }
    }

    #[inline]
    pub async fn finish(&mut self) -> Result<()> {
        self.send.finish().await
    }
}

impl AsyncRead for BiStream {
    #[inline]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for BiStream {
    #[inline]
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.send).poll_write(cx, buf)
    }

    #[inline]
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.send).poll_write_vectored(cx, bufs)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        self.send.is_write_vectored()
    }

    #[inline]
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}

pub struct IncomingUniStreams {
    incoming: QuinnConnection,
    reg: Registry,
}

impl IncomingUniStreams {
    #[inline]
    pub fn new(incoming: QuinnConnection, reg: Registry) -> Self {
        Self { incoming, reg }
    }
}

impl Stream for IncomingUniStreams {
    type Item = StdResult<RecvStream, ConnectionError>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(reg) = self.reg.get_register() {
            let mut fut = self.incoming.accept_uni();
            match unsafe { Pin::new_unchecked(&mut fut) }.poll(cx) {
                Poll::Ready(Ok(recv)) => Poll::Ready(Some(Ok(RecvStream::new(recv, reg)))),
                Poll::Ready(Err(ConnectionError::LocallyClosed)) => Poll::Ready(None),
                Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
                Poll::Pending => Poll::Pending,
            }
        } else {
            // the connection is already dropped
            Poll::Ready(None)
        }
    }
}

#[derive(Clone)]
pub struct Register(Arc<()>);

impl Register {
    #[inline]
    pub fn new() -> Self {
        Self(Arc::new(()))
    }

    #[inline]
    pub fn get_registry(&self) -> Registry {
        Registry(Arc::downgrade(&self.0))
    }

    #[inline]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.0)
    }
}

pub struct Registry(Weak<()>);

impl Registry {
    #[inline]
    pub fn get_register(&self) -> Option<Register> {
        self.0.upgrade().map(Register)
    }
}
