#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]
#![feature(total_cmp)]

pub mod api;
pub mod node_client;

#[macro_use]
extern crate rocket;
extern crate rocket_cors;
extern crate serde;
extern crate anyhow;
extern crate solana_sdk;
extern crate solana_client;
extern crate reqwest;

use rocket::http::Method;
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/get_blockhash")]
fn get_blockhash() -> String {
    api::get_blockhash()
}

#[get("/send_tx?<tx>")]
fn send_tx(tx: String) -> &'static str {
    api::send_tx(&tx);
    "success"
}

#[get("/simulate?<tx>")]
fn simulate_tx(tx: String) -> &'static str {
    api::simulate_tx(&tx);
    "success"
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index,get_blockhash,send_tx,simulate_tx])
        .launch();
}

fn get_cors() -> Cors {
    let allowed_origins = AllowedOrigins::All;
    rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post, Method::Options].into_iter()
            .map(From::from).collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }.to_cors().expect("cors config error")
}


