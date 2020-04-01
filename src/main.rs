use base64::decode;
use ed25519_dalek::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult};
use std::process::Command;
use warp::http::StatusCode;
use warp::{reject, Filter, Rejection, Reply};

use pretty_env_logger;

#[macro_use]
extern crate log;

#[derive(Deserialize, Serialize)]
struct Peer {
    public_key: String,
    address: [i8; 4],
}

#[derive(Deserialize, Serialize)]
struct Server {
    private_key: String,
    port: i16,
    address: [i8; 4],
}

#[derive(Deserialize, Serialize)]
struct Payload {
    server: Server,
    peers: Vec<Peer>,
}

#[derive(Deserialize, Serialize)]
struct Message {
    message: String,
    signature: String,
}

#[derive(Debug)]
struct IncorrectSignature;
impl reject::Reject for IncorrectSignature {}

#[derive(Debug)]
struct FailWritingConfig;
impl reject::Reject for FailWritingConfig {}

#[derive(Debug)]
struct FailRestartingWireguard;
impl reject::Reject for FailRestartingWireguard {}

// JSON replies

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "";
    } else if let Some(IncorrectSignature) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = "Not authorized to update configuration";
    } else if let Some(FailWritingConfig) = err.find() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Could not write config. Please check the server logs";
    } else if let Some(FailRestartingWireguard) = err.find() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Could not restart wireguard. Please check the server logs";
    } else {
        // We should have expected this... Just log and say its a 500
        error!("Unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });
    Ok(warp::reply::with_status(json, code))
}

fn decode_public_key_base64(public_key_base64: String) -> PublicKey {
    let mut raw_public_key_buffer = [0; PUBLIC_KEY_LENGTH];
    let raw_public_key_vector = base64::decode(&public_key_base64).unwrap();
    let raw_public_key_bytes = &raw_public_key_vector[..raw_public_key_buffer.len()];
    raw_public_key_buffer.copy_from_slice(raw_public_key_bytes);
    let decoded_public_key = PublicKey::from_bytes(&raw_public_key_buffer).unwrap();
    decoded_public_key
}

fn decode_signature_base64(signature_base64: String) -> Signature {
    let mut raw_signature_buffer = [0; SIGNATURE_LENGTH];
    let raw_signature_vector = base64::decode(&signature_base64).unwrap();
    let raw_signature_bytes = &raw_signature_vector[..raw_signature_buffer.len()];
    raw_signature_buffer.copy_from_slice(raw_signature_bytes);
    let decoded_signature = Signature::from_bytes(&raw_signature_buffer).unwrap();
    decoded_signature
}

fn get_key() -> String {
    let key = "PUBLIC_KEY";
    match env::var(key) {
        Ok(val) => return val,
        Err(_) => panic!("No public key was specified"),
    }
}

fn ok() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Copy {
    warp::get().and(warp::path!("ok").map(|| format!("OK")))
}

fn write_config_file(config: String) -> IOResult<()> {
    let file_name = "/etc/wireguard/server.conf";

    // TODO: change to File::with_options when it is stable
    match OpenOptions::new().write(true).open(file_name) {
        Ok(mut file) => {
            file.write_all(config.as_bytes())?;
            return Ok(());
        }
        Err(e) => match e.kind() {
            IOErrorKind::NotFound => {
                let mut file = File::create(file_name)?;
                file.write_all(config.as_bytes())?;
                Ok(())
            }
            _ => {
                return Err(e);
            }
        },
    }
}

fn restart_wireguard() -> IOResult<()> {
    match Command::new("systemctl")
        .arg("restart")
        .arg("wg-quick@server")
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                return Ok(());
            } else {
                match String::from_utf8(output.stdout) {
                    Ok(err) => {
                        return Err(IOError::new(IOErrorKind::Other, err));
                    }
                    Err(e) => return Err(IOError::new(IOErrorKind::Other, e)),
                }
            }
        }
        Err(e) => {
            return Err(e);
        }
    };
}

fn update(
    public_key: PublicKey,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Copy {
    warp::post()
        .and(warp::path("update"))
        .and(warp::body::json())
        // TODO: try to extract the signature verification into its own Filter
        .and(warp::any().map(move || public_key.clone()))
        .and_then(|message: Message, public_key: PublicKey| async move {
            let signature = decode_signature_base64(message.signature);
            let message_as_bytes = message.message.as_bytes();
            if public_key.verify(&message_as_bytes, &signature).is_ok() {
                Ok(message.message)
            } else {
                Err(reject::custom(IncorrectSignature))
            }
        })
        .and_then(|config: String| async {
            match write_config_file(config) {
                Ok(_) => match restart_wireguard() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        error!("Error when restarting Wireguard: {}", e);
                        return Err(reject::custom(FailRestartingWireguard));
                    }
                },
                Err(e) => {
                    error!("Error when writing config file: {}", e);
                    return Err(reject::custom(FailWritingConfig));
                }
            };
        })
        .map(|_| format!("Config updated"))
}

#[tokio::main]
async fn main() {
    let public_key_base64 = get_key();
    info!("Loaded public key: {}", public_key_base64);
    let public_key = decode_public_key_base64(public_key_base64);

    // Setup server
    pretty_env_logger::init();
    let log = warp::log("wirt::api");

    let cors = warp::cors()
        .allow_origin("https://wirt.network")
        .allow_methods(vec!["POST"])
        .allow_header("content-type");

    let update_options = warp::options().and(warp::path("update")).map(warp::reply);

    let routes = ok()
        .or(update(public_key))
        .or(update_options)
        .with(log)
        .with(cors)
        .recover(handle_rejection);
    // TODO: It should be possible to configure the port and host

    let port: String = env::var("PORT").unwrap_or("3030".into());
    let port: u16 = port.parse().unwrap();
    let host: Vec<u8> = env::var("HOST")
        .unwrap_or("0.0.0.0".into())
        .split(".")
        .collect::<Vec<&str>>()
        .iter()
        .map(|value| value.to_string())
        .map(|value| value.parse().expect("Invalid Hostname specified"))
        .collect();

    let host: [u8; 4] = [host[0], host[1], host[2], host[3]];

    warp::serve(routes).run((host, port)).await
}

#[tokio::test]
async fn test_ok() {
    let filter = ok();

    let response = warp::test::request().path("/ok").reply(&filter).await;
    assert_eq!(response.body(), "OK");
}
