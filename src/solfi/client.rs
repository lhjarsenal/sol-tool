use std::str::FromStr;
use crate::solfi::MarketAccount;
use anyhow::Result;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::RpcFilterType,
};
use solana_sdk::pubkey::Pubkey;

const SOLFI_PROGRAM_ID: &str = "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe";

pub fn fetch_all_market_accounts(rpc_client: &RpcClient) -> Result<Vec<(Pubkey, MarketAccount)>> {
    let market_accounts = rpc_client.get_program_accounts_with_config(
        &Pubkey::from_str(SOLFI_PROGRAM_ID).unwrap(),
        RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(
                size_of::<MarketAccount>() as u64
            )]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::JsonParsed),
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let market_accounts = market_accounts
        .into_iter()
        .map(|(pubkey, account)| {
            (
                pubkey,
                *MarketAccount::load(&mut account.data.as_slice()).unwrap(),
            )
        })
        .collect::<Vec<(Pubkey, MarketAccount)>>();
    Ok(market_accounts)
}

pub fn fetch_live_markets_accounts(rpc_client: &RpcClient) -> Result<Vec<(Pubkey, MarketAccount)>> {
    let market_accounts = fetch_all_market_accounts(rpc_client)?;
    println!("market_accounts_total={:?}", market_accounts.len());
    let live_market_accounts = market_accounts
        .into_iter()
        .filter(|(_, market)| market.market_config.enabled != 0)
        .collect::<Vec<(Pubkey, MarketAccount)>>();
    Ok(live_market_accounts)
}

pub fn get_canonical_market_account_address(
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"market", base_mint.as_ref(), quote_mint.as_ref()],
        &Pubkey::from_str(SOLFI_PROGRAM_ID).unwrap(),
    )
}

pub fn get_market_account_address_with_bump(
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    bump: u8,
) -> Option<Pubkey> {
    Pubkey::create_program_address(
        &[b"market", base_mint.as_ref(), quote_mint.as_ref(), &[bump]],
        &Pubkey::from_str(SOLFI_PROGRAM_ID).unwrap(),
    )
        .ok()
}
