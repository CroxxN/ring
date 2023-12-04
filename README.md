# Ring (Oxidized-Ping)

---

## What is Ring?

Ring is a command-line utility written in Rust that can send ICMP Echo Request Packets to a 
destination. Ring works like the ubiquitous `Ping` utility. Unlike Ping, Ring supports terminal
colors too. 

## How to use?

- Download the latest release from the `release` tab, and add it to your `path`.

- To ring a destination, just `ring <dest_address`. For example, to Ring google.com, simple use `ring google.com`.

- Use `CTLR + C` to stop ringing at any time.

## Options

-h, --help        Print this help message
-v, --version     Print current version
-4, --ipv4        Ring IPV4 host
-6, --ipv6        Ring IPV6 host
-b, --broadcast   Enable ringing broadcast address
-c, --count       Ring <n> times
-i, --interval    Ring every <n> seconds
-d, --timeout     Wait atmost <n> seconds for echo replies
-q, --quiet       Don't print intermediate ring results
-t, --ttl         Set time-to-live value

## TODO

- [ ] Adaptive ring
- [ ] Audible ring
- [ ] Accurate TTL value
- [ ] Extraction of IPv6 headers
- [ ] recvmsg
