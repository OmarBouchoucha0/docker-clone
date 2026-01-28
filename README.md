# docker-clone

A minimal container runtime written in **Rust** for learning purposes.  
This project shows how containers work internally by relying directly on Linux features.

## What this project does

- Isolates processes using **Linux namespaces**
- Changes the root filesystem using **pivot_root**(not yet)
- Limits resources with **cgroups v2** (CPU, memory, PIDs)
- Runs a command inside a lightweight container environment


## How to run

```bash
cargo build
systemd-run --user --scope -p Delegate=yes \
  ./target/debug/docker-clone run ./rootfs /bin/sh

