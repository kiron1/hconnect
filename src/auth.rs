use std::io::BufReader;
use std::{convert::Infallible, fs::File};

use base64::Engine;
use cross_krb5::{ClientCtx, InitiateFlags};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("file error {0}: {1}")]
    File(std::path::PathBuf, std::io::Error),
    #[error("netrc parser error: {0:?}")]
    Netrc(netrc::Error),
    #[error("negotiation error: {0}")]
    Negotiate(String), // Use a string, since cross_krb5 uses anyhow.
    #[error("no entry: {0}")]
    NoEntry(String),
}

#[derive(Debug, Clone)]
pub struct NetrcPath(std::path::PathBuf);

impl NetrcPath {
    pub fn open(&self) -> Result<Netrc, Error> {
        let netrc = File::open(&self.0).map_err({
            let p = self.0.clone();
            move |e| Error::File(p, e)
        })?;

        let netrc = netrc::Netrc::parse(BufReader::new(netrc)).map_err(Error::Netrc)?;

        Ok(Netrc(netrc))
    }
}

impl Default for NetrcPath {
    fn default() -> Self {
        let mut netrc = dirs::home_dir().unwrap_or_default();
        netrc.push(".netrc");
        Self(netrc)
    }
}

impl std::str::FromStr for NetrcPath {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(std::path::PathBuf::from(s)))
    }
}

impl std::fmt::Display for NetrcPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

#[derive(Debug)]
pub struct Netrc(netrc::Netrc);

impl Netrc {
    pub fn for_host(&self, host: &str) -> Result<Option<Token>, Error> {
        if let Some((_, machine)) = self.0.hosts.iter().find(|&x| x.0 == host) {
            let token = if let Some(ref password) = machine.password {
                format!("{}:{}", machine.login, password)
            } else {
                machine.login.to_string()
            };
            let token = base64::engine::general_purpose::STANDARD.encode(&*token);
            let token = Token(format!("Basic {}", token));
            Ok(Some(token))
        } else {
            Err(Error::NoEntry(host.to_owned()))
        }
    }
}

#[derive(Debug)]
pub struct Negotiate;

impl Negotiate {
    pub fn for_host(&self, host: &str) -> Result<Option<Token>, Error> {
        let principal = format!("HTTP/{}", host);

        let (_pending, token) = ClientCtx::new(InitiateFlags::empty(), None, &principal, None)
            .map_err(|e| Error::Negotiate(e.to_string()))?;
        let token = base64::engine::general_purpose::STANDARD.encode(&*token);
        let token = Token(format!("Negotiate {}", token));
        Ok(Some(token))
    }
}

#[derive(Debug, Clone)]
pub struct Token(String);

impl Token {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub enum Authenticator {
    None,
    Netrc(Netrc),
    Negotiate(Negotiate),
}

impl Authenticator {
    pub fn none() -> Self {
        Self::None
    }
    pub fn netrc(netrc: Netrc) -> Self {
        Self::Netrc(netrc)
    }

    pub fn negotiate() -> Self {
        Self::Negotiate(Negotiate)
    }

    pub fn for_host(&self, host: &str) -> Result<Option<Token>, Error> {
        match self {
            Self::None => Ok(None),
            Self::Netrc(n) => n.for_host(host),
            Self::Negotiate(n) => n.for_host(host),
        }
    }
}
