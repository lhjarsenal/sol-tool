use std::borrow::BorrowMut;
use std::str::FromStr;
use solana_client::rpc_client::RpcClient;
use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::clock::Epoch;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::commitment_config::CommitmentConfig;
use crate::node_client::NetworkType;
use crate::raydium::state::{AmmInfo, AmmStatus};
use crate::raydium::math::Calculator;
use crate::raydium::instruction::{AmmInstruction, SwapInstructionBaseIn};
use crate::raydium::error::AmmError;
use crate::raydium::processor::Processor;


pub fn process_swap_base_in() -> ProgramResult {
    let swap_base_in = AmmInstruction::SwapBaseIn(SwapInstructionBaseIn { amount_in: 1000000000, minimum_amount_out: 0 });
    let mut keys: Vec<Pubkey> = vec![];
    ///   0. `[]` Spl Token program id
    ///   1. `[writable]` AMM Account
    ///   2. `[]` $authority derived from `create_program_address(&[AUTHORITY_AMM, &[nonce]])`.
    ///   3. `[writable]` AMM open orders Account
    ///   4. `[writable]` (optional)AMM target orders Account, no longer used in the contract, recommended no need to add this Account.
    ///   5. `[writable]` AMM coin vault Account to swap FROM or To.
    ///   6. `[writable]` AMM pc vault Account to swap FROM or To.
    ///   7. `[]` Market program id
    ///   8. `[writable]` Market Account. Market program is the owner.
    ///   9. `[writable]` Market bids Account
    ///   10. `[writable]` Market asks Account
    ///   11. `[writable]` Market event queue Account
    ///   12. `[writable]` Market coin vault Account
    ///   13. `[writable]` Market pc vault Account
    ///   14. '[]` Market vault signer Account
    ///   15. `[writable]` User source token Account.
    ///   16. `[writable]` User destination token Account.
    ///   17. `[singer]` User wallet Account
    keys.push(Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap());
    keys.push(Pubkey::from_str("4NqJNZQr5ffSdtHw5XscEeooxY3qQuhMH29uW64Unfpw").unwrap());
    keys.push(Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap());
    keys.push(Pubkey::from_str("3MCWGwnqwiD3GD66Z1fecUp3tvZfKcVV8cmdMubwwv7B").unwrap());
    keys.push(Pubkey::from_str("43Tq5hjpLhv5E9bgPfU5u4r1KX7xHkwDW1QkER5JJjbN").unwrap());
    keys.push(Pubkey::from_str("CASbAUSvV7UD8YZwXNhNoXUzFZwuLL5ZzKejcWc6qKXf").unwrap());
    keys.push(Pubkey::from_str("8GCnUxibvoryxGAHRQFBfVJ4Q45KJqiJ2Ch77V7ECfUM").unwrap());
    keys.push(Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX").unwrap());
    keys.push(Pubkey::from_str("73w3gyKCsv2fLrgmNmuEPxXUrZ7x4qYE7cP1CJnRQD8y").unwrap());
    keys.push(Pubkey::from_str("3w2i4zndJNi7PG9LcxPQvwvpW4hBVzZZpbk9KzX3YwNy").unwrap());
    keys.push(Pubkey::from_str("5uqgg4LQyPS9Zgv2NzvNkntUJv1NUrNHR7JVN9fLN5YA").unwrap());
    keys.push(Pubkey::from_str("8BD92LdmNY6Z8pGF5eenHSYxMms8HzKL7BtaMpFAqkj").unwrap());
    keys.push(Pubkey::from_str("B9Bt2UKPvQuoQqHyviFTGN7aVusXBJPnfX89pwc3Fe6X").unwrap());
    keys.push(Pubkey::from_str("CwLvUFyDpq8p14kb81He6uf2PGdqo4nZkc9eohxzy5PN").unwrap());
    keys.push(Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap());
    keys.push(Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap());
    keys.push(Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap());
    keys.push(Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap());

    let client = RpcClient::new(NetworkType::MainTx.url().to_string());
    let accounts: Vec<Option<Account>> = client.get_multiple_accounts(&keys).unwrap();

    let k0 = accounts.get(0).unwrap().clone().unwrap().owner.clone();
    let l0 = &mut accounts.get(0).unwrap().clone().unwrap().lamports.clone();
    let d0 = &mut accounts.get(0).unwrap().clone().unwrap().data.clone();
    let e0 = accounts.get(0).unwrap().clone().unwrap().rent_epoch.clone();

    let k1 = accounts.get(1).unwrap().clone().unwrap().owner.clone();
    let l1 = &mut accounts.get(1).unwrap().clone().unwrap().lamports.clone();
    let d1 = &mut accounts.get(1).unwrap().clone().unwrap().data.clone();
    let e1 = accounts.get(1).unwrap().clone().unwrap().rent_epoch.clone();
    println!("data_length={:?}",d1.len());

    let k2 = accounts.get(2).unwrap().clone().unwrap().owner.clone();
    let l2 = &mut accounts.get(2).unwrap().clone().unwrap().lamports.clone();
    let d2 = &mut accounts.get(2).unwrap().clone().unwrap().data.clone();
    let e2 = accounts.get(2).unwrap().clone().unwrap().rent_epoch.clone();

    let k3 = accounts.get(3).unwrap().clone().unwrap().owner.clone();
    let l3 = &mut accounts.get(3).unwrap().clone().unwrap().lamports.clone();
    let d3 = &mut accounts.get(3).unwrap().clone().unwrap().data.clone();

    let k4 = accounts.get(4).unwrap().clone().unwrap().owner.clone();
    let l4 = &mut accounts.get(4).unwrap().clone().unwrap().lamports.clone();
    let d4 = &mut accounts.get(4).unwrap().clone().unwrap().data.clone();

    let k5 = accounts.get(5).unwrap().clone().unwrap().owner.clone();
    let l5 = &mut accounts.get(5).unwrap().clone().unwrap().lamports.clone();
    let d5 = &mut accounts.get(5).unwrap().clone().unwrap().data.clone();

    let k6 = accounts.get(6).unwrap().clone().unwrap().owner.clone();
    let l6 = &mut accounts.get(6).unwrap().clone().unwrap().lamports.clone();
    let d6 = &mut accounts.get(6).unwrap().clone().unwrap().data.clone();

    let k7 = accounts.get(7).unwrap().clone().unwrap().owner.clone();
    let l7 = &mut accounts.get(7).unwrap().clone().unwrap().lamports.clone();
    let d7 = &mut accounts.get(7).unwrap().clone().unwrap().data.clone();

    let k8 = accounts.get(8).unwrap().clone().unwrap().owner.clone();
    let l8 = &mut accounts.get(8).unwrap().clone().unwrap().lamports.clone();
    let d8 = &mut accounts.get(8).unwrap().clone().unwrap().data.clone();

    let k9 = accounts.get(9).unwrap().clone().unwrap().owner.clone();
    let l9 = &mut accounts.get(9).unwrap().clone().unwrap().lamports.clone();
    let d9 = &mut accounts.get(9).unwrap().clone().unwrap().data.clone();

    let k10 = accounts.get(10).unwrap().clone().unwrap().owner.clone();
    let l10 = &mut accounts.get(10).unwrap().clone().unwrap().lamports.clone();
    let d10 = &mut accounts.get(10).unwrap().clone().unwrap().data.clone();

    let k11 = accounts.get(11).unwrap().clone().unwrap().owner.clone();
    let l11 = &mut accounts.get(11).unwrap().clone().unwrap().lamports.clone();
    let d11 = &mut accounts.get(11).unwrap().clone().unwrap().data.clone();

    let k12 = accounts.get(12).unwrap().clone().unwrap().owner.clone();
    let l12 = &mut accounts.get(12).unwrap().clone().unwrap().lamports.clone();
    let d12 = &mut accounts.get(12).unwrap().clone().unwrap().data.clone();

    let k13 = accounts.get(13).unwrap().clone().unwrap().owner.clone();
    let l13 = &mut accounts.get(13).unwrap().clone().unwrap().lamports.clone();
    let d13 = &mut accounts.get(13).unwrap().clone().unwrap().data.clone();

    let k14 = accounts.get(14).unwrap().clone().unwrap().owner.clone();
    let l14 = &mut accounts.get(14).unwrap().clone().unwrap().lamports.clone();
    let d14 = &mut accounts.get(14).unwrap().clone().unwrap().data.clone();

    let k15 = accounts.get(15).unwrap().clone().unwrap().owner.clone();
    let l15 = &mut accounts.get(15).unwrap().clone().unwrap().lamports.clone();
    let d15 = &mut accounts.get(15).unwrap().clone().unwrap().data.clone();

    let k16 = accounts.get(16).unwrap().clone().unwrap().owner.clone();
    let l16 = &mut accounts.get(16).unwrap().clone().unwrap().lamports.clone();
    let d16 = &mut accounts.get(16).unwrap().clone().unwrap().data.clone();

    let k17 = accounts.get(17).unwrap().clone().unwrap().owner.clone();
    let l17 = &mut accounts.get(17).unwrap().clone().unwrap().lamports.clone();
    let d17 = &mut accounts.get(17).unwrap().clone().unwrap().data.clone();
    let infos = [
        AccountInfo::new(&keys.get(0).unwrap(), false, false, l0, d0, &k0, false, e0),
        AccountInfo::new(&keys.get(1).unwrap(), false, false, l1, d1, &k1, false, e1),
        AccountInfo::new(&keys.get(2).unwrap(), false, false, l2, d2, &k2, false, e2),
        AccountInfo::new(&keys.get(3).unwrap(), false, false, l3, d3, &k3, false, 0),
        AccountInfo::new(&keys.get(4).unwrap(), false, false, l4, d4, &k4, false, 0),
        AccountInfo::new(&keys.get(5).unwrap(), false, false, l5, d5, &k5, false, 0),
        AccountInfo::new(&keys.get(6).unwrap(), false, false, l6, d6, &k6, false, 0),
        AccountInfo::new(&keys.get(7).unwrap(), false, false, l7, d7, &k7, false, 0),
        AccountInfo::new(&keys.get(8).unwrap(), false, false, l8, d8, &k8, false, 0),
        AccountInfo::new(&keys.get(9).unwrap(), false, false, l9, d9, &k9, false, 0),
        AccountInfo::new(&keys.get(10).unwrap(), false, false, l10, d10, &k10, false, 0),
        AccountInfo::new(&keys.get(11).unwrap(), false, false, l11, d11, &k11, false, 0),
        AccountInfo::new(&keys.get(12).unwrap(), false, false, l12, d12, &k12, false, 0),
        AccountInfo::new(&keys.get(13).unwrap(), false, false, l13, d13, &k13, false, 0),
        AccountInfo::new(&keys.get(14).unwrap(), false, false, l14, d14, &k14, false, 0),
        AccountInfo::new(&keys.get(15).unwrap(), false, false, l15, d15, &k15, false, 0),
        AccountInfo::new(&keys.get(16).unwrap(), false, false, l16, d16, &k16, false, 0),
        AccountInfo::new(&keys.get(17).unwrap(), false, false, l17, d17, &k17, false, 0),
    ];

    let program_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
    Processor::process(&program_id, &infos, swap_base_in.pack().unwrap().as_slice());

    Ok(())
}

pub fn convert_to_info<'a>(key: &'a Pubkey, account: &'a mut Account) -> AccountInfo<'a> {
    AccountInfo::new(key,
                     false, false,
                     &mut account.lamports,
                     &mut account.data,
                     &account.owner, false,
                     Epoch::default())
}