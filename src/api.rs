use std::str::FromStr;
use bincode::serialize;
use crate::node_client::NetworkType;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding, UiTransactionEncoding};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcSendTransactionConfig, RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig};
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
use rocket::response::content::Json;
use solana_client::client_error::ClientErrorKind;
use solana_rpc_client_api::{
    client_error::{
        Error as ClientError, Result as ClientResult,
    }};
use serde::{Deserialize, Serialize};
use solana_account_decoder::UiAccountEncoding;
use solana_program::hash::Hash;
use solana_transaction_status::UiTransactionEncoding::JsonParsed;

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
    let tx = tx_encode.decode().unwrap();

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
    println!("lookup={:?}", address_lookup_table_account);

    let private_key = String::from("2pexWv8c7UGshhg565N3qdvV1qxLuZ19yyzxGB95cgHCWsL7yMPyYUBoHBMURypkLFCr3wFQrQ4WzwZ9rWxuW1FC");
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
        address_lookup_table_account
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
    let encoding = UiTransactionEncoding::Base58;
    let serialized_encoded = serialize_and_encode(&tx, encoding).unwrap();
    println!("tx_base58={:?}", serialized_encoded);

    let result = client.simulate_transaction_with_config(&tx, config);
    println!("simulate={:?}", result);
}

pub fn get_account() {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());

    //jupter address lookup
    let address_lookup_table_key = Pubkey::from_str("CUhicobqg7htGE8XNn7n11d8k4b6jTWdifnvzQ2qrDcj").unwrap();
    let config = RpcAccountInfoConfig{
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: None,
        min_context_slot: None
    };
    let account = client.get_account_with_config(&address_lookup_table_key,config).unwrap().value.unwrap();
    let address_lookup_table = AddressLookupTable::deserialize(&account.data).unwrap();
    let address_lookup_table_account = AddressLookupTableAccount {
        key: address_lookup_table_key,
        addresses: address_lookup_table.addresses.to_vec(),
    };
    println!("data={:?}", &account.data);
    println!("base64={:?}", Json(address_lookup_table_account));



}

fn serialize_and_encode<T>(input: &T, encoding: UiTransactionEncoding) -> ClientResult<String>
    where
        T: serde::ser::Serialize,
{
    let serialized = serialize(input)
        .map_err(|e| ClientErrorKind::Custom(format!("Serialization failed: {e}")))?;
    let encoded = match encoding {
        UiTransactionEncoding::Base58 => bs58::encode(serialized).into_string(),
        _ => {
            return Err(ClientErrorKind::Custom(format!(
                "unsupported encoding: {encoding}. Supported encodings: base58, base64"
            ))
                .into());
        }
    };
    Ok(encoded)
}