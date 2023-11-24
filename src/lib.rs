use std::net::SocketAddr;

use bytes::Bytes;
use http::header::{HOST, PROXY_AUTHORIZATION};
use http::Request;
use http::Uri;
use http_body_util::Empty;
use hyper::client::conn::http1;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

pub mod auth;

pub mod io_ext;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/o error: {0}")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),
    #[error("HTTP error: {0}")]
    Http(
        #[from]
        #[source]
        http::Error,
    ),
    #[error("HTTP wire error: {0}")]
    Hyper(
        #[from]
        #[source]
        hyper::Error,
    ),
    #[error("invalid URI: {0}")]
    InvalidUri(Uri),
    #[error("connection error when connecting to {0}: {1}")]
    Connect(Host, tokio::io::Error),
    #[error("handshake error with {0} via {1}: {2}")]
    Handshake(Host, SocketAddr, #[source] hyper::Error),
    #[error("authentication error: {0}")]
    Auth(
        #[from]
        #[source]
        crate::auth::Error,
    ),
}

#[derive(Debug)]
pub struct Connection {
    io: Upgraded,
}

impl Connection {
    pub async fn connect(
        proxy: Uri,
        target_uri: Uri,
        authorization: auth::Authenticator,
    ) -> Result<Connection, Error> {
        let proxy = Host::from_uri(&proxy)?;
        let target = Host::from_uri(&target_uri)?;

        let stream = TcpStream::connect(proxy.addr()).await.map_err({
            let p = proxy.clone();
            move |e| Error::Connect(p, e)
        })?;

        let proxy_addr = stream.peer_addr()?;

        let (mut request_sender, connection) =
            http1::handshake(TokioIo::new(stream)).await.map_err({
                let t = target.clone();
                move |e| Error::Handshake(t, proxy_addr, e)
            })?;

        let request = Request::builder()
            .method("CONNECT")
            .uri(&target_uri)
            .header(HOST, target.addr().0);
        let request = if let Some(token) = authorization.for_host(&proxy.0)? {
            request.header(PROXY_AUTHORIZATION, token.as_str())
        } else {
            request
        };
        let request = request.body(Empty::<Bytes>::new())?;

        let send_request = async move {
            let response = request_sender.send_request(request).await?;
            hyper::upgrade::on(response).await
        };

        let (response, _connection) = tokio::join!(send_request, connection.with_upgrades());

        Ok(Self { io: response? })
    }

    pub async fn copy_bidirectional<T>(self, mut other: T) -> std::io::Result<(u64, u64)>
    where
        T: AsyncRead + AsyncWrite + std::marker::Unpin,
    {
        let mut stream = Box::pin(TokioIo::new(self.io));
        tokio::io::copy_bidirectional(&mut stream, &mut other).await
    }
}

#[derive(Debug, Clone)]
pub struct Host(String, u16);

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

impl Host {
    fn from_uri(uri: &Uri) -> Result<Host, Error> {
        let authority = uri.authority();
        let host = authority.map(|a| a.host()).ok_or_else({
            let u = uri.to_owned();
            move || Error::InvalidUri(u)
        })?;
        let port = authority.and_then(|a| a.port_u16()).ok_or_else({
            let u = uri.to_owned();
            move || Error::InvalidUri(u)
        })?;

        Ok(Host(host.into(), port))
    }

    fn addr(&self) -> (String, u16) {
        (self.0.clone(), self.1)
    }
}
