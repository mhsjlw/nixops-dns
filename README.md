nixops-dns
==========

DNS Resolver for NixOps machines using [trust-dns](https://github.com/bluejekyll/trust-dns). I really *wouldn't* recommend using this for anything other than a local development environment... but my usecase is hosting a DNS server for a few other developers that have access to the NixOps state database and want quick access to testing/staging/production servers

## Usage
Get Rust installed, then just clone and build

```
git clone https://github.com/mhsjlw/nixops-dns
cd nixops-dns
cargo run --release
# In another terminal
dig +short machine @127.0.0.1 -p 5300
```

If you want to use it with dnsmasq, it's also quite easy. Just add:
```
server=/./127.0.0.1#5300
```

And your system will bounce requests down to the server!

Inspired by https://github.com/kamilchm/nixops-dns (but, uh, it's in Go)