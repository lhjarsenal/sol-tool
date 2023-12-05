use crate::node_client::NetworkType;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use serde_json::json;

pub fn get_blockhash() -> String {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let recent_blockhash = client.get_latest_blockhash().unwrap();
    recent_blockhash.to_string()
}

pub fn send_tx(tx: &str) {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let rpc_request = RpcRequest::SendTransaction;

    let config = RpcSendTransactionConfig {
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        ..RpcSendTransactionConfig::default()
    };

    let result: String = client.send(rpc_request, json!([tx,config])).unwrap();
    println!("result={}", result);
}

pub fn simulate_tx(tx: &str) {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());

    let tx_encode = EncodedTransaction::Binary(tx.to_string(), TransactionBinaryEncoding::Base58);
    let simulate_tx = tx_encode.decode().unwrap();
    let transaction = simulate_tx.into_legacy_transaction().unwrap();
    let result = client.simulate_transaction(&transaction);
    println!("simulate={:?}", result);
}


