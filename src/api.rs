use std::hash::Hash;
use std::str::FromStr;
use bincode::serialize;
use serde::Serialize;
use crate::node_client::NetworkType;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding, UiTransactionEncoding};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig};
use solana_sdk::commitment_config::CommitmentLevel;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::{SanitizedVersionedTransaction, VersionedTransaction};
use solana_sdk::message::{v0, VersionedMessage, v0::Message};
use solana_program::address_lookup_table::state::AddressLookupTable;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::system_instruction::SystemInstruction;
use retry::{delay::Exponential, retry};

pub const SOLANA_SYSTEM_ID: &str = "11111111111111111111111111111111";

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

pub fn simulate_tx(tx: &str) -> Vec<String> {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());

    let tx_encode = EncodedTransaction::Binary(tx.to_string(), TransactionBinaryEncoding::Base58);
    let simulate_tx = tx_encode.decode().unwrap();
    let mut transaction = simulate_tx.into_legacy_transaction().unwrap();

    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: None,
        encoding: None,
        accounts: Some(RpcSimulateTransactionAccountsConfig {
            encoding: None,
            addresses: vec![],
        }),
        min_context_slot: None,
    };

    let result = client.simulate_transaction_with_config(&transaction, config);
    println!("simulate={:?}", result);
    let r = result.unwrap().value;
    r.logs.unwrap()
}

pub fn send_v0_demo() {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());

    //jupter address lookup
    let address_lookup_table_key = Pubkey::from_str("4Qk1tYQsBWghy3P6Vi5x99pbhFWyvgohbJy7xUYu2EoZ").unwrap();
    let raw_account = client.get_account(&address_lookup_table_key).unwrap();
    let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data).unwrap();
    let address_lookup_table_account = AddressLookupTableAccount {
        key: address_lookup_table_key,
        addresses: address_lookup_table.addresses.to_vec(),
    };
    println!("lookup={:?}",address_lookup_table_account);

    let private_key = String::from("private");
    let keypair = Keypair::from_base58_string(&private_key);
    //创建一个lookup demo
    let user = Pubkey::from_str("3cUbuUEJkcgtzGxvsukksNzmgqaUK9jwFS5pqRpoevtN").unwrap();
    let receive = Pubkey::from_str("FrgCk25LzUB9aGdf4c4xbtG79JK4Wtuvnkp1XLfQJzF5").unwrap();

    let transfer_ix = Instruction::new_with_bincode(
        Pubkey::from_str(SOLANA_SYSTEM_ID).unwrap(),
        &SystemInstruction::Transfer { lamports: 10000000 },
        vec![
            AccountMeta::new(user, true),
            AccountMeta::new(receive, false),
        ],
    );

    let instructions = vec![transfer_ix];

    let address_lookup_table_accounts = vec![
        AddressLookupTableAccount {
            key: address_lookup_table_key,
            addresses: vec![],
        }
    ];

    let recent_blockhash = client.get_latest_blockhash().unwrap();

    let message = Message::try_compile(
        &user,
        &instructions,
        &address_lookup_table_accounts,
        recent_blockhash,
    ).unwrap();

    let tx = VersionedTransaction {
        signatures: vec![keypair.sign_message(&message.serialize())],
        message: VersionedMessage::V0(message),
    };
    // let res = retry(
    //     Exponential::from_millis_with_factor(250, 2.0).take(3),
    //     || client.send_and_confirm_transaction(&tx),
    // );
    // let sig = res.unwrap();
    // println!("Tx sig: {:?}", sig);

    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: None,
        encoding: None,
        accounts: Some(RpcSimulateTransactionAccountsConfig {
            encoding: None,
            addresses: vec![],
        }),
        min_context_slot: None,
    };

    let result = client.simulate_transaction_with_config(&tx, config);
    println!("simulate={:?}", result);
}

