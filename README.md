# hconnect

`hconnect` can establish a TCP connection to a host behind a proxy. It is
similar to [`corkscrew`][corkscrew] or [`nc -Xconnect -x...`][ncx], _but_ can
authenticate against a proxy using the [basic][basic] or [negotiate][negotiate]
via [Kerberos][kerberos] (using the [GSS-API](gssapi) Linux and macOS or
[SSPI][sspi] on Windows) authorization method 

## Usage

The following command will establish a TCP connection with the host behind the
proxy `proxy.exmaple.com` listening on port `8080`. When the proxy responds
with _407 Proxy Authentication Required_, when the file `~/.netrc` exists
`hconnect` will consult it for an entry for the given post host. If no such
entry can be found or the file does not exists, `hconnect` will try to generate
a Kerberos token.

```sh
hconnect proxy.example.com:8080 %h:%p
```

### SSH

Place the following fragment in your [`~/.ssh/config`][sshconfig] file:

```
ProxyCommand hconnect proxy.example.com:8080 %h:%p
```

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
