# Server adapter for Wirt

This adapter is responsible for:

- listening to incomming requests from a users [WirtUI](https://wirt.network)
- automatically update the WireGuard® configuration with new values

in this order.

To guarantee that only the user who owns the server can update it, the WirtUI public key has to be provided when running the adapter. With this it is possible verify incomming payloads.

More info at https://wirt.network/docs/setup

## Setup

### Prerequisites

In order to use this service your server must support `systemd` to control processes.

### Installation

#### Compilation

- Install rust and cargo with https://rustup.rs/ or your preferred installation method.
- `cargo build --release`

### Using the wirt-adapter

- Place the `target/release/wirt-adapter` executable from your compilation, or a release from the [release page](https://github.com/wirtnetwork/adapter/releases) into `/usr/bin` or `/bin` on your server.
- Generate the public key from your [wirtUI](https://wirt.network/settings) webapp
- Create a new user that is allowed to change the config at `/etc/wireguard/server.conf`
- Copy the example `wirt-adapter.service` config to `/etc/systemd/system/` and update it with the correct values
- Start and enable the service with `systemctl enable --now wirt-adapter`

#### Automatically restarting wireguard

- install `inotify-tools`
- Place the `wireguard-restarter.sh` script to `/usr/bin` and make it executable (`chmod +x /usr/bin/wireguard-restarter.sh`)
- Copy the example `wireguard-restarter.service` config to `/etc/systemd/system/` and update it with the correct values
- Start and enable the service with `systemctl enable --now wireguard-restarter`

### Configuring your wirt adapter instance

The wirt adapter has to be configured using environment variables:

- **PUBLIC_KEY**: Public key of your https://wirt.network app
- **SSL_PEM_CERT**: SSL certificate
- **SSL_KEY**: SSL private key for the certificate

#### Optional

- **HOST**: the host address to listen on
- **PORT**: the port to listen on
- **CONFIG_PATH**: path to your Wireguard configuration

## Contributing

Simply check the `src/main.rs` file for **TODO** comments, to see what can be optimized in the code.

If you have other ideas please open a PR for small improvements.

For bigger changes, lets discuss these in an issue first.

### Other TODOs

- Testing needs to be done. One sample test to show how to test filters is implemented

## Development

Always run the server with a supplied public key. You can generate a fake one with `wg genkey` or simply use `wKcOYzxjaQV03gIy0uBov+WgeR1U20XY8o63r1ljuUc=`.

### Localhost and CORS

Make sure to send requests with the origin set to `https://wirt.network` or update the allowed origin with `ALLOWED_ORIGIN` as an environment variable.

### Continuous Testing

Run `cargo watch -x test`

### Logging

To enable logging run the program with the wanted environment variable.
Check [env-logger](https://docs.rs/env_logger/0.7.1/env_logger/) for more information.

#### Server logs

`RUST_LOG=wirt::api`. For example `RUST_LOG=wirt::api cargo run`

#### Info logs

`RUST_LOG=info`
