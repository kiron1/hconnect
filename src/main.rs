use clap::{ArgAction, Parser};
use hconnect::io_ext::Unsplit;
use hconnect::{auth, Connection};
use http::Uri;

/// Establish a TCP connection through a proxy
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, action = ArgAction::Count)]
    /// Enable more verbose output
    verbose: u8,
    #[clap(long)]
    /// Force usage of basic authorization
    netrc: bool,
    #[clap(long, default_value_t = Default::default())]
    /// Netrc file path
    netrc_file: auth::NetrcPath,
    #[clap(long)]
    /// Force usage of negotiate authorization
    negotiate: bool,
    #[clap(long, value_parser = parse_duration, default_value = "30")]
    /// Maximum time in seconds that hconnect will use for the connection phase
    connect_timeout: std::time::Duration,
    /// Proxy address
    #[clap(long, env = "http_proxy")]
    proxy: Uri,
    /// Target address
    target: Uri,
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, std::num::ParseFloatError> {
    let seconds = arg.parse()?;
    Ok(std::time::Duration::from_secs_f32(seconds))
}

impl<T: ?Sized> FutureExt for T where T: std::future::Future {}

pub trait FutureExt: std::future::Future {
    fn timeout(self, dt: std::time::Duration) -> tokio::time::Timeout<Self>
    where
        Self: Sized,
    {
        tokio::time::timeout(dt, self)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let auth = if args.netrc {
        auth::Authenticator::netrc(args.netrc_file.open()?)
    } else if args.negotiate {
        auth::Authenticator::negotiate()
    } else {
        auth::Authenticator::none()
    };

    let h = Connection::connect(args.proxy, args.target, auth)
        .timeout(args.connect_timeout)
        .await??;

    let stdio = Unsplit::new(tokio::io::stdin(), tokio::io::stdout());
    let (bytesin, bytesout) = h.copy_bidirectional(stdio).await?;

    if args.verbose > 0 {
        eprintln!("transfered {bytesin} byes in / {bytesout} bytes out")
    }

    Ok(())
}
