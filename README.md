# hconnect

[![main](https://github.com/kiron1/hconnect/actions/workflows/main.yaml/badge.svg)](https://github.com/kiron1/hconnect/actions/workflows/main.yaml)
[![Crates.io](https://img.shields.io/crates/v/hconnect)](https://crates.io/crates/hconnect)

`hconnect` can establish a TCP connection to a host behind a proxy. It is
similar to [`corkscrew`][corkscrew] or [`nc -Xconnect -x...`][ncx], _but_ can
authenticate against a proxy using the [basic][basic] or [negotiate][negotiate]
via [Kerberos][kerberos] (using the [GSS-API](gssapi) Linux and macOS or
[SSPI][sspi] on Windows) authorization method

## Usage examples

Below are different usage examples of `hconnect`. Replace `%h` with the host
behind the proxy and `%p` with the port.

### No authentication

The following command will establish a TCP connection with the host behind the
proxy `proxy.exmaple.com` listening on port `8080`.

```sh
hconnect --proxy proxy.example.com:8080 %h:%p
```

Proxies which require authentication, will response with: _407 Proxy
Authentication Required_. In this case we can either use Basic authentication by
consulting the `~/.netrc` file with the following command:

### Basic authentication

```sh
hconnect --netrc --proxy proxy.example.com:8080 %h:%p
```

In the command above, the `.netrc` file from the defualt locatoin in your
`$HOME` directory will be used. A custom path for the `.netrc` file can be
specified by using the `--netrc-file NETRC_PATH` argument. The `.netrc` file
will need an entry like this:

```
machine proxy.example.com
login USERNAME
password PASSWORD
```

The value for `machine` must match with the proxy host (in this example
`proxy.example.com`). The `USERNAME` and `PASSWORD` must be adjusted
accordingly.

### Negotiate

The best option for authentication is via `--negotiate` since in this way no
additional configuration is requied and no password needs to be stored or
transmitted (neither in plain text nor encrypted).

```sh
hconnect --negotiate --proxy proxy.example.com:8080 %h:%p
```

### SSH

Place the following fragment in your [`~/.ssh/config`][sshconfig] file:

```
ProxyCommand hconnect --proxy proxy.example.com:8080 %h:%p
```

Add either `--netrc` or `--negotiate` if authentication is required. The `ssh`
command will automatically replace `%h` and `%p` with the SSH target host and
port.

## License

This source code is under the [MIT](https://opensource.org/licenses/MIT) license
with the exceptions mentioned in "Third party source code in this repository".

[corkscrew]: https://github.com/bryanpkc/corkscrew "Corkscrew is a tool for tunneling SSH through HTTP proxies"
[ncx]: https://man.openbsd.org/nc#X "nc - arbitrary TCP and UDP connections and listens"
[basic]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication#basic_authentication_scheme "Basic authentication scheme"
[negotiate]: https://datatracker.ietf.org/doc/html/rfc4559.html#section-4 "HTTP Negotiate Authentication Scheme"
[gssapi]: https://web.mit.edu/kerberos/krb5-devel/doc/appdev/gssapi.html "Generic Security Services API (GSSAPI)"
[sspi]: https://docs.microsoft.com/en-us/windows/win32/rpc/security-support-provider-interface-sspi- "Security Support Provider Interface (SSPI)"
[kerberos]: https://datatracker.ietf.org/doc/html/rfc4120 "The Kerberos Network Authentication Service (V5)"
[sshconfig]: https://man.openbsd.org/ssh_config "ssh_config - OpenSSH client configuration file"
