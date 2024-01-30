#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]
#![feature(total_cmp)]

pub mod api;
pub mod node_client;
pub mod raydium;
pub mod pool_test;

#[macro_use]
extern crate rocket;
extern crate rocket_cors;
extern crate serde;
extern crate anyhow;
extern crate solana_sdk;
extern crate solana_client;
extern crate reqwest;
extern crate solana_program;
extern crate serum_dex;
extern crate num_traits;
extern crate uint;
extern crate spl_token;
extern crate spl_associated_token_account;
extern crate bytemuck;
extern crate safe_transmute;
extern crate thiserror;
extern crate bincode;

use rocket::http::Method;
use rocket_contrib::json::Json;
use rocket_cors::{Cors, AllowedOrigins, AllowedHeaders};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateResponse {
    pub logs: Vec<String>,
}

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
fn simulate_tx(tx: String) -> Json<SimulateResponse> {
    let logs = api::simulate_tx(&tx);
    Json(SimulateResponse {
        logs
    })
}

fn main() {
    // pool_test::process_swap_base_in();
    // api::get_account();
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


