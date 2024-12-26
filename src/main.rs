#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]
#![feature(total_cmp)]

pub mod api;
pub mod node_client;
pub mod raydium;
pub mod pool_test;
pub mod solfi;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockSlotResponse {
    pub hash: String,
    pub slot: u64
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/get_blockhash")]
fn get_blockhash() -> String {
    api::get_blockhash()
}

#[get("/get_hash_and_slot")]
fn get_hash_and_slot() -> Json<BlockSlotResponse> {
    let block = api::get_hash_and_slot();
    Json(BlockSlotResponse {
        hash: block.0,
        slot: block.1
    })
}

#[get("/get_slot")]
fn get_slot() -> String {
    api::get_slot()
}

#[get("/send_tx?<tx>")]
fn send_tx(tx: String) -> String {
    let hash = api::send_tx(&tx);
    hash
}

#[get("/close?<account>")]
fn close(account: String) -> String {
    let hash = api::close(&account);
    hash
}

#[get("/simulate?<tx>")]
fn simulate_tx(tx: String) -> Json<SimulateResponse> {
    let logs = api::simulate_tx(&tx);
    Json(SimulateResponse {
        logs
    })
}

fn main() {
    // api::get_solfi_accounts();
    api::get_solfi_account();
    // rocket::ignite()
    //     .mount("/", routes![index,get_blockhash,send_tx,simulate_tx,get_hash_and_slot,get_slot,close])
    //     .launch();
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


