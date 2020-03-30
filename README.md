# Server adapter for Wirt

This adapter is responsible for:

- listening to incomming requests from a users Wirt application
- automatically update the Wireguard configuration with new values
- restart the Wireguard server.

in this order.

To guarantee that only the user who owns the server can update it, the Wirt app has to be connected to the server via key-based authentification.

## Setup

### Prerequisits

In order to use this service your server must support `systemd` to control processes.

### HowTo

- Create a new user that is allowed to change the config and restart the Server with `systemd`.
- Place the server into `/usr/bin`.
- Copy the example `systemd` config to `/etc/systemd/system/` and update it with the correct values
- Start and enable the service with `systemctl enable --now wirt-adapter`

## Development

### Continuous Testing

Run `cargo watch -x test`

### Logging

To enable logging run the program with the wanted environment variable.
Check [env-logger](https://docs.rs/env_logger/0.7.1/env_logger/) for more information.

#### Server logs

`RUST_LOG=wirt::api`. For example `RUST_LOG=wirt::api cargo run`

#### Info logs

`RUST_LOG=info`

## TODO

- Write payload structs
- Implement listening
- Correctly identify a client
- Parse input for validity
- Update config
- Restart server
- Return status code
- Write `systemd` service
- Write developer docs
