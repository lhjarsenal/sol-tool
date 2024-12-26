use std::str::FromStr;
use bincode::serialize;
use crate::node_client::NetworkType;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding, UiTransactionEncoding};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcSendTransactionConfig, RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig};
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer};
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::Transaction;
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
use solana_sdk::address_lookup_table::instruction::{close_lookup_table, extend_lookup_table, deactivate_lookup_table};
use crate::solfi;
use solfi::client;
use crate::solfi::MarketAccount;

pub const SOLANA_SYSTEM_ID: &str = "11111111111111111111111111111111";

pub fn get_blockhash() -> String {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let recent_blockhash = client.get_latest_blockhash().unwrap();
    recent_blockhash.to_string()
}

pub fn get_hash_and_slot() -> (String, u64) {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let config = CommitmentConfig::confirmed();
    let block = client.get_latest_blockhash_with_commitment(config).unwrap();
    let slot = client.get_slot_with_commitment(config).unwrap();
    (block.0.to_string(), u64::from_str(&slot.to_string()).unwrap())
}

pub fn get_slot() -> String {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let config = CommitmentConfig::confirmed();
    let slot = client.get_slot_with_commitment(config).unwrap();
    slot.to_string()
}

pub fn send_tx(tx: &str) -> String {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let rpc_request = RpcRequest::SendTransaction;

    let config = RpcSendTransactionConfig {
        preflight_commitment: Some(CommitmentLevel::Confirmed),
        ..RpcSendTransactionConfig::default()
    };

    let result: String = client.send(rpc_request, json!([tx,config])).unwrap();
    result
}

pub fn close(account: &str) -> String {
    let private_key = String::from("x");
    let keypair = Keypair::from_base58_string(&private_key);

    let out = close_dev(&keypair, account);
    out
}

pub fn close_dev(
    keypair: &Keypair,
    account: &str,
) -> String {
    let close = Pubkey::from_str(account).unwrap();
    let payer = Pubkey::from_str("x").unwrap();
    // let deactivate_ix = deactivate_lookup_table(close,payer);
    // let extend_ix=extend_lookup_table(close,payer,None,vec![payer]);
    let close_ix = close_lookup_table(close, payer, payer);
    let block = get_hash_and_slot();
    let recent_blockhash = Hash::from_str(&block.0).unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[close_ix],
        Some(&payer),
        &[keypair],
        recent_blockhash,
    );

    let tx_bytes = bincode::serialize(&tx).unwrap();
    let base58 = bs58::encode(tx_bytes).into_string();
    println!("base58={:?}", base58);
    let result = send_tx(&base58);
    result
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
        inner_instructions: false,
    };

    let result = client.simulate_transaction_with_config(&tx, config);
    println!("simulate={:?}", result);
    let r = result.unwrap().value;
    r.logs.unwrap()
}

pub fn debug_base58_v0() {
    //2eDTJaGbcUWAy3aKLwWdSeRS3JdJ9ZbfHhhLQgLR4jSLgHVKS6vrJ3Wn3fCKEnkyVR495cHNneAGtrnfjBHa5uJ6i5TEAEY2ASy7ZMHbwrXGac4MiBGHNtTjjyVMhoeZ9kxGvSvy988pkpQVxpGggsbdp2rs8nGFdyDZtWAbzpeAakWxsWDAnthSp49WhMSH3bTjje8CYpTirfFwQCqAvJtLek5TwntsiqxShDmfHcNAz3JieqYYwY9CDPjPJuZnsFQ4z84HjrLLmCnJz6GweZLmoQ9mJRrS8ouBbkFeiCw1
    let tx = String::from("8LmwrtepEBJyU4ybQSTEZkQ3vJ5RRyFaQxKqJGiGpSZ4rWqPcLKoB4YGzSNgmUsqM6yPyY7p1e7W9eTgs1HWVR3opjqykiwSLqxhfuKFjHt9DDFCKmyBCPRjEx8RwssTw9nYizuzQr9ANcAM78C9VpB7Psjkz485r9rna2eyCnSQKSwsJhu6jwHe4tRFLzsyMkXLhKjKaMGD1sGLjvHeeVDto7nuqbHoPKU7qhvoYajvdwC9JPAq9TCBg2Y6TX7SaTnoqLTJW7W5PUd8SFv4EYeT9gMtCthx56UMujFMD8PitG45cDSXxuiiXaVQ85hB3XVCWwWR1mjX9CrWUSc2wAjcPZWdBifNxUtCbowJ3rjHrh5D1jUPi2ZjdneAiznWCJBx4AsTtzamLSZpTbeAec1JoWGWFcjrMAW9h4sCE2UqzvbGAjRM371XyNbJPi7LbhddnSUeQZAS5WsvdKDyC26Jbwp5nRg79iknSDVZ83D61DPycuRQZXiTQsNYgyyE4bTSqPXJVRF4mW9uT9NGEZR96K7xFa1o6ULCoqp7fMY9KiRqLAbTHydzVrAATw8AX9F8cbdW4fevn26p7aVECe65PFUDqr8bvbuW12FtH4BZJyfWP31QCTGNDUnVqhqNfv5BunT99SFtx57ujunF8L5vNt9TQSYFWhD6QGtHYYU5XhpkoA46f9ep8XB6Rj1Z2N5DteVEnd5oeDeV2kCRvGbioGgKcUEvfsQZA5yjjU7oLHyPuKvu5qxm4hrK5vAMsnuSXxv3ubueTZp6TFqAWefjevXLXWAnn5DFhB2n9Pd1PB7XvLNmM2UVxr8X5CGk43XZdVTJ9F4EraQVQqqHwFY5fMr44gRfkpCzopZSB6v4VxTnSHfP81bTeGiii63QKyU1Lnnb8E4NvA5qxjNbiS9XLBHpzNPxby9BiGF34fzmT1hZZRE8NHTDfYUAG17aXvL485mFfTTVwd7dxMV59z9VyX7JkMUxALR7YD1tb32J2tSYHNcxF9PVxkzFc4qYCETQQ2SeoGcy8qwL6SX6XwLLXCwV3dUAWYmL1B7B3uzBspDT91qbNmUttJxz5dygqk4FcbfUUtGg7nUUBSWeVP7HKeqw4X44cBdhT2MazbnLjbqj8XJ4Y1aNqUgM9oZ3TgkBGoU8B3X5VcTg5QoytRCjsn4832qoqDKeAY9CBCb6jbtqTuxqNf33uHrLagsxFbtngzesaEZnAVBFBcdnEZmRj7LLfav3tU1uhXbrk8tqFTzAh3nfNEtDVmjKaBNcijVLrLjBaLPc4FSc7yW3PiDDHN7tGLABboy55VXYSmkiXY8Q69ZfRni673HwNGqvpmrffuH913aTFDoqoTReAbHJ2QX7FSB5TkSG15ZcshBBWjzYhm2ndSz53A4ZRBcuA5nBwNejns3itj6u557oETRpYFpCCY2Yu4jemTW8SyezGbuxrvg6BdcuhUvdpzM6zVyPkZgQuTWyA1rWsKEk2yu7EJV2ub1Whtamn2kjoNCHzZTHspx5uoogLQ3CWsMLHM9kev2YXa8pstiVdvK53yAKcsaCwzBsiLiL3rT5cxNcMCiPmSaibEC8CpJVKN4Ph2qT1o1ZMkva5xLx53SZPmnvazsq2gUN2bDVEXzJjMeA3boR9XkMikBR8pP1QMerkYrarUsajNKtVQ3kRkj");

    let tx_encode = EncodedTransaction::Binary(tx.to_string(), TransactionBinaryEncoding::Base58);
    let tx = tx_encode.decode().unwrap();
    println!("tx={:?}", tx);

    let tx_bytes = bincode::serialize(&tx).unwrap();
    println!("tx_bytes={:?}", tx_bytes);
    let base58 = bs58::encode(tx_bytes).into_string();
    println!("base58={:?}", base58);
}

pub fn send_v0_demo() {
    let client = RpcClient::new(NetworkType::MainTx.url().to_string());

    //jupter address lookup
    let address_lookup_table_key = Pubkey::from_str("4jgg9CHLiTeQUwSDK9srby9Vp1NhDGqpdacWvUASGUwY").unwrap();
    let raw_account = client.get_account(&address_lookup_table_key).unwrap();
    let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data).unwrap();
    let address_lookup_table_account = AddressLookupTableAccount {
        key: address_lookup_table_key,
        addresses: address_lookup_table.addresses.to_vec(),
    };
    println!("lookup={:?}", address_lookup_table_account);

    let private_key = String::from("x");
    let keypair = Keypair::from_base58_string(&private_key);
    //创建一个lookup demo
    let user = Pubkey::from_str("x").unwrap();
    let receive = Pubkey::from_str("x").unwrap();

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
        inner_instructions: false,
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
    let config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: None,
        min_context_slot: None,
    };
    let account = client.get_account_with_config(&address_lookup_table_key, config).unwrap().value.unwrap();
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

pub fn get_solfi_accounts() {
    let rpc_client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let accounts = client::fetch_live_markets_accounts(&rpc_client).unwrap();
    println!("live accounts lens={:?}", accounts.len());

    for i in 0..accounts.len() {
        let account = accounts.get(i).unwrap();
        println!("pubkey={:?}", account.0);
        println!("account={:?}", account.1);
        println!("-------------------------------------");
    }


}

pub fn get_solfi_account() {
    let sol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
    let market_account = Pubkey::from_str("CAPhoEse9xEH95XmdnJjYrZdNCA8xfUWdy3aWymHa1Vj").unwrap();

    let account = client::get_canonical_market_account_address(&sol,&usdc).0;
    let market_account = client::get_market_account_address_with_bump(&sol,&usdc,246).unwrap();
    println!("account={:?}",account);
    println!("market_account={:?}",market_account);
}