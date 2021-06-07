[![Crates.io](https://img.shields.io/crates/v/rustdns.svg)](https://crates.io/crates/rustdns)
[![Documentation](https://docs.rs/rustdns/badge.svg)](https://docs.rs/rustdns)
[![Build Status](https://github.com/bramp/rustdns/actions/workflows/rust.yml/badge.svg)](https://github.com/bramp/rustdns)

# rustdns

rustdns is a simple, fast, and fully fledged DNS library for interacting
with domain name services at a high or low level.

## Features
* Parsing and generating the following record types:
  * A,
  * AAAA,
  * CNAME,
  * MX,
  * NS,
  * SOA,
  * PTR,
  * TXT, and
  * SRV
* Extension Mechanisms for DNS ([EDNS(0)]).
* Support [International Domain Names (IDNA)](https://en.wikipedia.org/wiki/Internationalized_domain_name) - Different scripts, alphabets, anhd even emojis!
* Sample `dig` style [command line](#usage-cli).
* Fully [tested](#testing), and [fuzzed](#fuzzing).

## Usage (low-level library)

```rust
use rustdns::Message;
use rustdns::types::*;
use std::net::UdpSocket;
use std::time::Duration;

fn udp_example() -> std::io::Result<()> {
    // A DNS Message can be easily constructed
    let mut m = Message::default();
    m.add_question("bramp.net", Type::A, Class::Internet);
    m.add_extension(Extension {   // Optionally add a EDNS extension
        payload_size: 4096,       // which supports a larger payload size.
        ..Default::default()
    });

    // Setup a UDP socket for sending to a DNS server.
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::new(5, 0)))?;
    socket.connect("8.8.8.8:53")?; // Google's Public DNS Servers

    // Encode the DNS Message as a Vec<u8>.
    let question = m.to_vec()?;

    // Send to the server.
    socket.send(&question)?;

    // Wait for a response from the DNS server.
    let mut resp = [0; 4096];
    let len = socket.recv(&mut resp)?;

    // Take the response bytes and turn it into another DNS Message.
    let answer = Message::from_slice(&resp[0..len])?;

    // Now do something with `answer`, in this case print it!
    println!("DNS Response:\n{}", answer);

    Ok(())
}
```

If successful something like the following will be printed:

```
;; ->>HEADER<<- opcode: Query, status: NoError, id: 44857
;; flags: qr rd ra ad; QUERY: 1, ANSWER: 2, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 512
;; QUESTION SECTION:
; bramp.net.              IN   A

; ANSWER SECTION:
bramp.net.            299 IN   A      104.21.62.200
bramp.net.            299 IN   A      172.67.138.196
```

## Usage (cli)

To use the [demo CLI](https://github.com/bramp/rustdns/blob/main/src/rustdns/main.rs):

```shell
$ cargo run A www.google.com
...
response: 8.8.8.8:53
00000000: 86 17 81 80 00 01 00 01 00 00 00 01 03 77 77 77  ..........www
00000010: 06 67 6F 6F 67 6C 65 03 63 6F 6D 00 00 01 00 01  .google.com.....
00000020: C0 0C 00 01 00 01 00 00 00 6E 00 04 8E FA 48 C4  À........n..úHÄ
00000030: 00 00 29 02 00 00 00 00 00 00 00                 ..)........

;; ->>HEADER<<- opcode: Query, status: NoError, id: 34327
;; flags: qr rd ra; QUERY: 1, ANSWER: 1, AUTHORITY: 0, ADDITIONAL: 1

;; OPT PSEUDOSECTION:
; EDNS: version: 0, flags:; udp: 512
;; QUESTION SECTION:
; www.google.com.         IN   A

; ANSWER SECTION:
www.google.com.       110 IN   A      142.250.72.196

$ cargo run AAAA www.google.com
$ cargo run ANY www.google.com
$ cargo run CNAME code.google.com
$ cargo run MX google.com
$ cargo run PTR 4.4.8.8.in-addr.arpa
$ cargo run SOA google.com
$ cargo run SRV _ldap._tcp.google.com
$ cargo run TXT google.com
```
## Testing

```shell
$ cargo test

# or the handy
$ cargo watch -- cargo test --lib -- --nocapture
```

The test suite is full of stored real life examples, from querying real DNS records.
This was generated with `cargo run -p generate_tests`.

### Fuzzing

The library has been extensively fuzzed. Try for yourself:

```shell
$ cargo fuzz run from_slice
```

### Releasing

```shell
$ cargo readme > README.md
$ cargo publish --dry-run
$ cargo publish
```

## TODO (in order of priority)
* [ ] UDP/TCP library
* [ ] Client side examples
* [ ] Server side examples
* [ ] DNS over TLS (DoT) and DNS over HTTPS (DoH)
* [ ] DNSSEC: Signing, validating and key generation for DSA, RSA, ECDSA and Ed25519
* [ ] RFC 1035 zone file parsing
* [ ] NSID, Cookies, AXFR/IXFR, TSIG, SIG(0)
* [ ] Refactoring to use <https://github.com/tokio-rs/bytes>


### Reference

* [rfc1034]: DOMAIN NAMES - CONCEPTS AND FACILITIES
* [rfc1035]: DOMAIN NAMES - IMPLEMENTATION AND SPECIFICATION
* [rfc6895]: Domain Name System (DNS) IANA Considerations
* [IANA Domain Name System (DNS) Parameters](https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml)
* [Computer Networks CPS365 FALL 2016](https://courses.cs.duke.edu//fall16/compsci356/DNS/DNS-primer.pdf)
* [miekg's Go DNS Library](https://github.com/miekg/dns)

[EDNS(0)]: https://en.wikipedia.org/wiki/Extension_Mechanisms_for_DNS
[rfc1034]: https://datatracker.ietf.org/doc/html/rfc1034
[rfc1035]: https://datatracker.ietf.org/doc/html/rfc1035
[rfc6895]: https://datatracker.ietf.org/doc/html/rfc6895

## License: Apache-2.0

```
Copyright 2021 Andrew Brampton (bramp.net)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
