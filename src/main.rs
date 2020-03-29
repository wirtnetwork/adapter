use serde::{Deserialize, Serialize};
use warp::Filter;

use std::fs::File;
use std::io::prelude::*;
use std::io::Result;

use pretty_env_logger;

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
    config: String,
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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let log = warp::log("wirt::api");

    let cors = warp::cors()
        .allow_origin("http://localhost:8080")
        .allow_methods(vec!["POST"])
        .allow_header("content-type");

    let update = warp::post()
        .and(warp::path("update"))
        .and(warp::body::json())
        .map(|message: Message| format!("{}", message.config));

    let update_options = warp::options().and(warp::path("update")).map(warp::reply);

    let routes = ok().or(update).or(update_options).with(log).with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[tokio::test]
async fn test_ok() {
    let filter = ok();

    let response = warp::test::request().path("/ok").reply(&filter).await;
    assert_eq!(response.body(), "OK");
}
