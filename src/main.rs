use bytes::Bytes;
use clap::Parser;
use http::{Request, StatusCode};
use hyper::{client::conn, Body};
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

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
    proxy: String,
    /// Target address
    target: String,
}

async fn tunnel<T>(io: T, bytes: Bytes) -> anyhow::Result<(u64, u64)>
where
    T: AsyncRead + AsyncWrite,
{
    let mut stdout = io::stdout();
    stdout.write_all(&bytes).await?;
    let (mut tcpout, mut tcpin) = io::split(io);
    let bytesout = io::copy(&mut tcpout, &mut stdout).await?;
    let bytesin = io::copy(&mut io::stdin(), &mut tcpin).await?;
    Ok((bytesin, bytesout))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let connect_timeout = std::time::Duration::from_secs_f32(args.connect_timeout);
    let target_stream = timeout(connect_timeout, TcpStream::connect(&args.proxy)).await??;

    let (mut request_sender, connection) =
        timeout(connect_timeout, conn::handshake(target_stream)).await??;

    // spawn a task to poll the connection and drive the HTTP state
    let task = tokio::spawn(async move {
        let parts = connection.without_shutdown().await;
        match parts {
            Ok(parts) => tunnel(parts.io, parts.read_buf).await,
            Err(cause) => Err(cause.into()),
        }
    });

    let request = Request::builder()
        .method("CONNECT")
        .uri(&args.target)
        .header("Host", args.target)
        .body(Body::from(""))?;

    let response = request_sender.send_request(request).await?;
    assert!(response.status() == StatusCode::OK);

    task.await??;

    Ok(())
}
