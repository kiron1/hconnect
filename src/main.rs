use std::fs::File;
use std::io::BufReader;

use anyhow::Context;
use bytes::Bytes;
use clap::Parser;
use cross_krb5::{ClientCtx, InitiateFlags};
use http::header::{HOST, PROXY_AUTHORIZATION};
use http::{Request, StatusCode, Uri};
use hyper::{client::conn, Body};
use netrc::Netrc;
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::try_join;
use tower::ServiceExt;

/// Establish a TCP connection through a proxy
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_occurrences))]
    /// Enable more verbose output
    verbose: usize,
    #[clap(long, default_value = "30")]
    /// Maximum time in seconds that hconnect will use for the connection phase
    connect_timeout: f32, //std::time::Duration,
    /// Proxy address
    proxy: Uri,
    /// Target address
    target: String,
}

pub async fn copy_and_close<'a, R, W>(reader: &'a mut R, writer: &'a mut W) -> std::io::Result<u64>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let res = tokio::io::copy(reader, writer).await;
    if res.is_ok() {
        writer.flush().await?;
        writer.shutdown().await?;
    }
    res
}

async fn tunnel<T>(io: T, bytes: Bytes) -> std::io::Result<(u64, u64)>
where
    T: AsyncRead + AsyncWrite,
{
    let mut stdout = io::stdout();
    let mut stdin = io::stdin();
    stdout.write_all(&bytes).await?;
    let (mut tcpout, mut tcpin) = io::split(io);
    let copyout = copy_and_close(&mut tcpout, &mut stdout);
    let copyin = copy_and_close(&mut stdin, &mut tcpin);
    try_join!(copyin, copyout)
}

fn netrc_authorization(host: &str) -> anyhow::Result<Option<String>> {
    let mut netrc = dirs::home_dir().unwrap_or_default();
    netrc.push(".netrc");

    // consider the ~/.netrc file as optional
    let file = match File::open(netrc) {
        Ok(file) => file,
        Err(_cause) => return Ok(None),
    };

    // but parser errors are real errors
    let netrc = Netrc::parse(BufReader::new(file))
        .map_err(|e| anyhow::anyhow!("netrc parser error: {:?}", e))?;

    if let Some(&(_, ref machine)) = netrc.hosts.iter().find(|&x| x.0 == host) {
        let token = if let Some(ref password) = machine.password {
            format!("{}:{}", machine.login, password)
        } else {
            machine.login.to_string()
        };
        let token = format!("Basic {}", base64::encode(&token));
        Ok(Some(token))
    } else {
        Ok(None)
    }
}

fn kerberos_authorization(host: &str) -> anyhow::Result<String> {
    let principal = format!("HTTP/{}", host);

    let client_ctx = ClientCtx::initiate(InitiateFlags::empty(), None, &principal);

    match client_ctx {
        Ok((_pending, token)) => {
            let b64token = base64::encode(&*token);
            let token = format!("Negotiate {}", b64token);
            Ok(token)
        }
        Err(cause) => Err(cause).context("proxy authorization"),
    }
}

fn authorization(host: &str) -> anyhow::Result<String> {
    // No netrc entry found, try Kerberos
    match netrc_authorization(host) {
        Ok(Some(token)) => Ok(token),
        Ok(None) => kerberos_authorization(host),
        Err(cause) => Err(cause),
    }
}

fn uri_to_addr(uri: &Uri) -> anyhow::Result<(String, u16)> {
    let authority = uri.authority();
    let host = authority
        .map(|a| a.host())
        .ok_or_else(|| anyhow::anyhow!("URI is invalid, missing host: {}", &uri))?;
    let port = authority.and_then(|a| a.port_u16()).unwrap_or(3128);

    Ok((host.into(), port))
}

enum Handshake {
    Success(TcpStream, Bytes),
    AuthenticationRequired,
}

async fn handshake<T: AsRef<str>>(
    (proxy_host, proxy_port): (T, u16),
    target: String,
    authorization: Option<String>,
) -> anyhow::Result<Handshake> {
    let stream = TcpStream::connect((proxy_host.as_ref(), proxy_port)).await?;

    let (mut request_sender, connection) = conn::handshake(stream).await?;

    // spawn a task to poll the connection and drive the HTTP state
    let task = tokio::spawn(async move {
        let parts = connection.without_shutdown().await;
        match parts {
            Ok(parts) => Ok((parts.io, parts.read_buf)),
            Err(cause) => Err(cause).context("connection"),
        }
    });

    let request = Request::builder()
        .method("CONNECT")
        .uri(&target)
        .header(HOST, target);

    let request = if let Some(auth) = authorization {
        request.header(PROXY_AUTHORIZATION, auth)
    } else {
        request
    };
    let request = request.body(Body::from(""))?;

    let response = request_sender.send_request(request).await?;

    if response.status() == StatusCode::OK {
        let _body = hyper::body::to_bytes(response.into_body()).await?;
        request_sender.ready().await?;
        let (io, read_buf) = task.await??;
        Ok(Handshake::Success(io, read_buf))
    } else if response.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED {
        let _body = hyper::body::to_bytes(response.into_body()).await?;
        request_sender.ready().await?;
        let (_io, _read_buf) = task.await??;
        Ok(Handshake::AuthenticationRequired)
    } else {
        Err(anyhow::anyhow!(
            "unexpected HTTP status code: {}",
            response.status()
        ))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let connect_timeout = std::time::Duration::from_secs_f32(args.connect_timeout);

    let proxy = uri_to_addr(&args.proxy)?;

    let h = timeout(
        connect_timeout,
        handshake(proxy.clone(), args.target.clone(), None),
    )
    .await??;

    let (io, buf) = match h {
        Handshake::Success(io, buf) => Ok((io, buf)),
        Handshake::AuthenticationRequired => {
            let auth = authorization(&proxy.0)?;
            let h = timeout(connect_timeout, handshake(proxy, args.target, Some(auth))).await??;
            match h {
                Handshake::Success(io, buf) => Ok((io, buf)),
                Handshake::AuthenticationRequired => {
                    Err(anyhow::anyhow!("authorization rejected by proxy"))
                }
            }
        }
    }?;

    let (_bytesin, _bytesout) = tunnel(io, buf).await.context("tunnel")?;

    Ok(())
}
