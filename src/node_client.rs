use std::error::Error;

use anyhow::Result;
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_client::{client_error::Result as ClientResult, rpc_client::RpcClient};
use solana_sdk::pubkey::Pubkey;

pub struct NetworkOpts {
    url: String,
}

pub enum NetworkType {
    Mainnet,
    MainTx,
    Devnet,
    DevTx,
    Serum,
    Custom(NetworkOpts),
}

impl NetworkType {
    pub fn url(&self) -> &str {
        match self {
            //https://solana--devnet.datahub.figment.io/apikey/e5eb3da747d0b7e884cec99e3973213e
            NetworkType::Devnet => "https://hk32.rpcpool.com",//https://psytrbhymqlkfrhudd.dev.genesysgo.net:8899   https://hk32.rpcpool.com
            NetworkType::DevTx => "https://solana--devnet.datahub.figment.io/apikey/e5eb3da747d0b7e884cec99e3973213e",
            NetworkType::Mainnet => "https://mainnet.rpcpool.com",
            NetworkType::MainTx => "https://solana-mainnet.g.alchemy.com/v2/DhpWoKr5-y16c687f07cQMu7B0KZq83a",
            NetworkType::Serum => "https://solana-api.projectserum.com",
            NetworkType::Custom(nework_opts) => &nework_opts.url,
        }
    }
}

pub fn get_rpc_client(network: &NetworkType) -> ClientResult<RpcClient> {
    let client = RpcClient::new(network.url().to_string());

    let version = client.get_version()?;
    println!("RPC version: {:?}", version);
    Ok(client)
}

pub struct Client {
    rpc_client: RpcClient,
    payer: Keypair,
}

impl Client {
    pub fn new(network_type: NetworkType, payer: Keypair, _path: &String) -> Result<Self, Box::<dyn Error>> {
        let client = get_rpc_client(&network_type)?;
        Ok(Client { rpc_client: client, payer })
    }

    pub fn rpc(&self) -> &RpcClient {
        &self.rpc_client
    }
    pub fn payer(&self) -> Pubkey {
        self.payer.pubkey()
    }
}

