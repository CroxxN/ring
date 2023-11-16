# Ring (Oxidized-Ping)

---

## What is Ring?

Ring is a command-line utility written in Rust that can send ICMP Echo Request Packets to a 
destination. Ring works like the ubiquitous `Ping` utility. Unlike Ping, Ring supports terminal
colors too. 

## How to use?

- Download the latest release from the `release` tab, and add it to your `path`.
- Ring requires `sudo` previlages because it needs to send raw packets. You can allow Ring superuser
previlages for a single time by `sudo ring <dest_address`, or, more comfortably, you can modify the permissions
of the Ring binary with `chmod u+s <path_to_ring>/ring[.exe]`.
- To ring a destination, just `ring <dest_address`. For example, to Ring google.com, simple use `ring google.com`.
- Use `CTLR + C` to stop ringing at any time.


