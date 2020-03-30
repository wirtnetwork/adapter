use serde::{Deserialize, Serialize};
use warp::{reject, Filter};

use base64::decode;
use ed25519_dalek::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;

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

fn load_key() -> Result<String> {
    // TODO: for testing generate a random file on disk for this
    let mut file = File::open("public.key")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn ok() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Copy {
    warp::get().and(warp::path!("ok").map(|| format!("OK")))
}

fn update(
    public_key: PublicKey,
) -> impl Filter<Extract = (String,), Error = warp::Rejection> + Copy {
    warp::post()
        .and(warp::path("update"))
        .and(warp::body::json())
        // The public key gets injected as a filter here
        .and(warp::any().map(move || public_key.clone()))
        .map(|message: Message, public_key: PublicKey| {
            let signature = decode_signature_base64(message.signature);
            let message = message.message.as_bytes();
            format!("Signature is: {}", {
                public_key.verify(&message, &signature).is_ok()
            })
        })
}

#[tokio::main]
async fn main() {
    let public_key_base64 =
        load_key().expect("Could not load public key. Make sure to save it into `public.key`");
    info!("Loaded public key: {}", public_key_base64);
    let public_key = decode_public_key_base64(public_key_base64);

    // Setup server
    pretty_env_logger::init();
    let log = warp::log("wirt::api");

    let cors = warp::cors()
        .allow_origin("http://localhost:8080")
        .allow_methods(vec!["POST"])
        .allow_header("content-type");

    let update_options = warp::options().and(warp::path("update")).map(warp::reply);

    let routes = ok()
        .or(update(public_key))
        .or(update_options)
        .with(log)
        .with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[tokio::test]
async fn test_ok() {
    let filter = ok();

    let response = warp::test::request().path("/ok").reply(&filter).await;
    assert_eq!(response.body(), "OK");
}
