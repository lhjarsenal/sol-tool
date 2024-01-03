use serum_dex::{
    critbit::{LeafNode, Slab, SlabView},
    matching::{OrderType, Side},
    state::{Market, MarketState, OpenOrders, ToAlignedBytes},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    // log::sol_log_compute_units,
    program_error::ProgramError,
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock, Sysvar},
};

use arrayref::{array_ref, array_refs};
use arrform::{arrform, ArrForm};
use bytemuck::from_bytes_mut;
use std::{
    cell::RefMut, collections::VecDeque, convert::identity, convert::TryFrom, mem::size_of,
    num::NonZeroU64, ops::Deref,
};
use crate::raydium::error::AmmError;
use crate::raydium::instruction::{AmmInstruction, InitializeInstruction2, SwapInstructionBaseIn};
use crate::raydium::math::{U128, Calculator, InvariantPool, RoundDirection, SwapDirection, U256, CheckedCeilDiv};
use crate::raydium::state::{AmmConfig, AmmInfo, AmmStatus, TargetOrders};

pub mod srm_token {
    solana_program::declare_id!("SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt");
}

pub mod msrm_token {
    solana_program::declare_id!("MSRMcoVyrFxnSgo5uXwone5SKcGhT1KEJMFEkMEWf9L");
}

#[cfg(feature = "localnet")]
pub mod config_feature {
    pub mod amm_owner {
        solana_program::declare_id!("75KWb5XcqPTgacQyNw9P5QU2HL3xpezEVcgsFCiJgTT");
    }

    pub mod openbook_program {
        solana_program::declare_id!("kGeitTdTHT1WdpUScdm8yxUAirZwbnQtqrpzvAm1p98");
    }

    pub mod referrer_pc_wallet {
        solana_program::declare_id!("75KWb5XcqPTgacQyNw9P5QU2HL3xpezEVcgsFCiJgTT");
    }

    pub mod create_pool_fee_address {
        solana_program::declare_id!("75KWb5XcqPTgacQyNw9P5QU2HL3xpezEVcgsFCiJgTT");
    }
}

#[cfg(feature = "devnet")]
pub mod config_feature {
    pub mod amm_owner {
        solana_program::declare_id!("Adm29NctkKwJGaaiU8CXqdV6WDTwR81JbxV8zoxn745Y");
    }

    pub mod openbook_program {
        solana_program::declare_id!("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj");
    }

    pub mod referrer_pc_wallet {
        solana_program::declare_id!("4NpMfWThvJQsV9VLjUXXpn3tPv1zoQpib8wCBDc1EBzD");
    }

    pub mod create_pool_fee_address {
        solana_program::declare_id!("3XMrhbv989VxAMi3DErLV9eJht1pHppW5LbKxe9fkEFR");
    }
}

#[cfg(not(any(feature = "localnet", feature = "devnet")))]
pub mod config_feature {
    pub mod amm_owner {
        solana_program::declare_id!("GThUX1Atko4tqhN2NaiTazWSeFWMuiUvfFnyJyUghFMJ");
    }

    pub mod openbook_program {
        solana_program::declare_id!("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
    }

    pub mod referrer_pc_wallet {
        solana_program::declare_id!("FCxGKqGSVeV1d3WsmAXt45A5iQdCS6kKCeJy3EUBigMG");
    }

    pub mod create_pool_fee_address {
        solana_program::declare_id!("7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5");
    }
}

/// Suffix for amm authority seed
pub const AUTHORITY_AMM: &'static [u8] = b"amm authority";
/// Suffix for amm associated seed
pub const AMM_ASSOCIATED_SEED: &'static [u8] = b"amm_associated_seed";
/// Suffix for target associated seed
pub const TARGET_ASSOCIATED_SEED: &'static [u8] = b"target_associated_seed";
/// Suffix for amm open order associated seed
pub const OPEN_ORDER_ASSOCIATED_SEED: &'static [u8] = b"open_order_associated_seed";
/// Suffix for coin vault associated seed
pub const COIN_VAULT_ASSOCIATED_SEED: &'static [u8] = b"coin_vault_associated_seed";
/// Suffix for pc vault associated seed
pub const PC_VAULT_ASSOCIATED_SEED: &'static [u8] = b"pc_vault_associated_seed";
/// Suffix for lp mint associated seed
pub const LP_MINT_ASSOCIATED_SEED: &'static [u8] = b"lp_mint_associated_seed";
/// Amm config seed
pub const AMM_CONFIG_SEED: &'static [u8] = b"amm_config_account_seed";

pub fn get_associated_address_and_bump_seed(
    info_id: &Pubkey,
    market_address: &Pubkey,
    associated_seed: &[u8],
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &info_id.to_bytes(),
            &market_address.to_bytes(),
            &associated_seed,
        ],
        program_id,
    )
}

/// Program state handler.
pub struct Processor {}

impl Processor {
    #[inline]
    fn check_account_readonly(account_info: &AccountInfo) -> ProgramResult {
        if account_info.is_writable {
            return Err(AmmError::AccountNeedReadOnly.into());
        }
        return Ok(());
    }

    /// Unpacks a spl_token `Account`.
    #[inline]
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Account, AmmError> {
        if account_info.owner != token_program_id {
            Err(AmmError::InvalidSplTokenProgram)
        } else {
            spl_token::state::Account::unpack(&account_info.data.borrow())
                .map_err(|_| AmmError::ExpectedAccount)
        }
    }

    /// Unpacks a spl_token `Mint`.
    #[inline]
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Mint, AmmError> {
        if account_info.owner != token_program_id {
            Err(AmmError::InvalidSplTokenProgram)
        } else {
            spl_token::state::Mint::unpack(&account_info.data.borrow())
                .map_err(|_| AmmError::ExpectedMint)
        }
    }

    fn load_orders<'a>(
        orders_account: &'a AccountInfo,
    ) -> Result<RefMut<'a, OpenOrders>, ProgramError> {
        let (_, data) = serum_dex::state::strip_header::<[u8; 0], u8>(orders_account, false)?;
        let open_orders: RefMut<'a, OpenOrders>;
        open_orders = RefMut::map(data, |data| from_bytes_mut(data));
        Ok(open_orders)
    }

    pub fn load_serum_market_order<'a>(
        market_acc: &AccountInfo<'a>,
        open_orders_acc: &AccountInfo<'a>,
        authority_acc: &AccountInfo<'a>,
        amm: &AmmInfo,
        // Allow for the market flag to be set to AccountFlag::Disabled
        allow_disabled: bool,
    ) -> Result<(Box<MarketState>, Box<OpenOrders>), ProgramError> {
        let market_state = Market::load(market_acc, &amm.market_program, allow_disabled)?;
        let open_orders = market_state.load_orders_mut(
            open_orders_acc,
            Some(authority_acc),
            &amm.market_program,
            None,
            None,
        )?;
        if identity(open_orders.market) != market_acc.key.to_aligned_bytes() {
            return Err(AmmError::InvalidMarket.into());
        }
        if identity(open_orders.owner) != authority_acc.key.to_aligned_bytes() {
            return Err(AmmError::InvalidOwner.into());
        }
        if *open_orders_acc.key != amm.open_orders {
            return Err(AmmError::InvalidOpenOrders.into());
        }
        return Ok((
            Box::new(*market_state.deref()),
            Box::new(*open_orders.deref()),
        ));
    }

    fn _get_dex_best_price(slab: &RefMut<serum_dex::critbit::Slab>, side: Side) -> Option<u64> {
        if slab.is_empty() {
            None
        } else {
            match side {
                Side::Bid => {
                    let best_bid_h = slab.find_max().unwrap();
                    let best_bid_px = slab
                        .get(best_bid_h)
                        .unwrap()
                        .as_leaf()
                        .unwrap()
                        .price()
                        .get();
                    Some(best_bid_px)
                }
                Side::Ask => {
                    let best_ask_h = slab.find_min().unwrap();
                    let best_ask_px = slab
                        .get(best_ask_h)
                        .unwrap()
                        .as_leaf()
                        .unwrap()
                        .price()
                        .get();
                    Some(best_ask_px)
                }
            }
        }
    }

    fn get_amm_orders(
        open_orders: &OpenOrders,
        bids: RefMut<Slab>,
        asks: RefMut<Slab>,
    ) -> Result<(Vec<LeafNode>, Vec<LeafNode>), ProgramError> {
        let orders_number = open_orders.free_slot_bits.count_zeros();
        let mut bids_orders: Vec<LeafNode> = Vec::new();
        let mut asks_orders: Vec<LeafNode> = Vec::new();
        if orders_number != 0 {
            for i in 0..128 {
                let slot_mask = 1u128 << i;
                if open_orders.free_slot_bits & slot_mask != 0 {
                    // means slot is free
                    continue;
                }
                if open_orders.is_bid_bits & slot_mask != 0 {
                    match bids.find_by_key(open_orders.orders[i]) {
                        None => continue,
                        Some(handle_bid) => {
                            let handle_bid_ref = bids.get(handle_bid).unwrap().as_leaf().unwrap();
                            bids_orders.push(*handle_bid_ref);
                        }
                    }
                } else {
                    match asks.find_by_key(open_orders.orders[i]) {
                        None => continue,
                        Some(handle_ask) => {
                            let handle_ask_ref = asks.get(handle_ask).unwrap().as_leaf().unwrap();
                            asks_orders.push(*handle_ask_ref);
                        }
                    }
                };
            }
        }
        bids_orders.sort_by(|a, b| b.price().get().cmp(&a.price().get()));
        asks_orders.sort_by(|a, b| a.price().get().cmp(&b.price().get()));
        Ok((bids_orders, asks_orders))
    }

    pub fn get_amm_best_price(
        market_state: &MarketState,
        open_orders: &OpenOrders,
        bids_account: &AccountInfo,
        asks_account: &AccountInfo,
    ) -> Result<(Option<u64>, Option<u64>), ProgramError> {
        let bids = market_state.load_bids_mut(&bids_account)?;
        let asks = market_state.load_asks_mut(&asks_account)?;
        let (bids_orders, asks_orders) = Self::get_amm_orders(open_orders, bids, asks)?;
        let mut bid_best_price = None;
        let mut ask_best_price = None;
        if bids_orders.len() != 0 {
            bid_best_price = Some(bids_orders.first().unwrap().price().get());
        }
        if asks_orders.len() != 0 {
            ask_best_price = Some(asks_orders.first().unwrap().price().get());
        }
        Ok((bid_best_price, ask_best_price))
    }

    pub fn get_amm_worst_price(
        market_state: &MarketState,
        open_orders: &OpenOrders,
        bids_account: &AccountInfo,
        asks_account: &AccountInfo,
    ) -> Result<(Option<u64>, Option<u64>), ProgramError> {
        let bids = market_state.load_bids_mut(&bids_account)?;
        let asks = market_state.load_asks_mut(&asks_account)?;
        let (bids_orders, asks_orders) = Self::get_amm_orders(open_orders, bids, asks)?;
        let mut bid_best_price = None;
        let mut ask_best_price = None;
        if bids_orders.len() != 0 {
            bid_best_price = Some(bids_orders.last().unwrap().price().get());
        }
        if asks_orders.len() != 0 {
            ask_best_price = Some(asks_orders.last().unwrap().price().get());
        }
        Ok((bid_best_price, ask_best_price))
    }

    /// The Detailed calculation of pnl
    /// 1. calc last_k witch dose not take pnl: last_k = calc_pnl_x * calc_pnl_y;
    /// 2. calc current price: current_price = current_x / current_y;
    /// 3. calc x after take pnl: x_after_take_pnl = sqrt(last_k * current_price);
    /// 4. calc y after take pnl: y_after_take_pnl = x_after_take_pnl / current_price;
    ///                           y_after_take_pnl = x_after_take_pnl * current_y / current_x;
    /// 5. calc pnl_x & pnl_y:  pnl_x = current_x - x_after_take_pnl;
    ///                         pnl_y = current_y - y_after_take_pnl;
    pub fn calc_take_pnl(
        target: &TargetOrders,
        amm: &mut AmmInfo,
        total_pc_without_take_pnl: &mut u64,
        total_coin_without_take_pnl: &mut u64,
        x1: U256,
        y1: U256,
    ) -> Result<(u128, u128), ProgramError> {
        // calc pnl
        let mut delta_x: u128;
        let mut delta_y: u128;
        if x1.checked_mul(y1).unwrap()
            >= (U256::from(target.calc_pnl_x))
            .checked_mul(target.calc_pnl_y.into())
            .unwrap()
        {
            // last k is
            // let last_k: u128 = (target.calc_pnl_x as u128).checked_mul(target.calc_pnl_y as u128).unwrap();
            // current k is
            // let current_k: u128 = (x1 as u128).checked_mul(y1 as u128).unwrap();
            // current p is
            // let current_p: u128 = (x1 as u128).checked_div(y1 as u128).unwrap();
            let x2_power = Calculator::calc_x_power(
                target.calc_pnl_x.into(),
                target.calc_pnl_y.into(),
                x1,
                y1,
            );
            // let x2 = Calculator::sqrt(x2_power).unwrap();
            let x2 = x2_power.integer_sqrt();
            // msg!(arrform!(LOG_SIZE, "calc_take_pnl x2_power:{}, x2:{}", x2_power, x2).as_str());
            let y2 = x2.checked_mul(y1).unwrap().checked_div(x1).unwrap();
            // msg!(arrform!(LOG_SIZE, "calc_take_pnl y2:{}", y2).as_str());

            // transfer to token_coin_pnl and token_pc_pnl
            // (x1 -x2) * pnl / sys_decimal_value
            let diff_x = U128::from(x1.checked_sub(x2).unwrap().as_u128());
            let diff_y = U128::from(y1.checked_sub(y2).unwrap().as_u128());
            delta_x = diff_x
                .checked_mul(amm.fees.pnl_numerator.into())
                .unwrap()
                .checked_div(amm.fees.pnl_denominator.into())
                .unwrap()
                .as_u128();
            delta_y = diff_y
                .checked_mul(amm.fees.pnl_numerator.into())
                .unwrap()
                .checked_div(amm.fees.pnl_denominator.into())
                .unwrap()
                .as_u128();

            let diff_pc_pnl_amount =
                Calculator::restore_decimal(diff_x, amm.pc_decimals, amm.sys_decimal_value);
            let diff_coin_pnl_amount =
                Calculator::restore_decimal(diff_y, amm.coin_decimals, amm.sys_decimal_value);
            let pc_pnl_amount = diff_pc_pnl_amount
                .checked_mul(amm.fees.pnl_numerator.into())
                .unwrap()
                .checked_div(amm.fees.pnl_denominator.into())
                .unwrap()
                .as_u64();
            let coin_pnl_amount = diff_coin_pnl_amount
                .checked_mul(amm.fees.pnl_numerator.into())
                .unwrap()
                .checked_div(amm.fees.pnl_denominator.into())
                .unwrap()
                .as_u64();
            if pc_pnl_amount != 0 && coin_pnl_amount != 0 {
                // step2: save total_pnl_pc & total_pnl_coin
                amm.state_data.total_pnl_pc = amm
                    .state_data
                    .total_pnl_pc
                    .checked_add(diff_pc_pnl_amount.as_u64())
                    .unwrap();
                amm.state_data.total_pnl_coin = amm
                    .state_data
                    .total_pnl_coin
                    .checked_add(diff_coin_pnl_amount.as_u64())
                    .unwrap();
                amm.state_data.need_take_pnl_pc = amm
                    .state_data
                    .need_take_pnl_pc
                    .checked_add(pc_pnl_amount)
                    .unwrap();
                amm.state_data.need_take_pnl_coin = amm
                    .state_data
                    .need_take_pnl_coin
                    .checked_add(coin_pnl_amount)
                    .unwrap();

                // step3: update total_coin and total_pc without pnl
                *total_pc_without_take_pnl = (*total_pc_without_take_pnl)
                    .checked_sub(pc_pnl_amount)
                    .unwrap();
                *total_coin_without_take_pnl = (*total_coin_without_take_pnl)
                    .checked_sub(coin_pnl_amount)
                    .unwrap();
            } else {
                delta_x = 0;
                delta_y = 0;
            }
        } else {
            return Err(AmmError::CalcPnlError.into());
        }

        Ok((delta_x, delta_y))
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        amm_seed: &[u8],
        nonce: u8,
    ) -> Result<Pubkey, AmmError> {
        Pubkey::create_program_address(&[amm_seed, &[nonce]], program_id)
            .map_err(|_| AmmError::InvalidProgramAddress.into())
    }

    pub fn process_swap_base_in(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        swap: SwapInstructionBaseIn,
    ) -> ProgramResult {
        const ACCOUNT_LEN: usize = 17;
        let input_account_len = accounts.len();

        let account_info_iter = &mut accounts.iter();
        let token_program_info = next_account_info(account_info_iter)?;

        let amm_info = next_account_info(account_info_iter)?;
        println!("amm_info={:?}",amm_info.key);
        println!("amm_info_owner={:?}",amm_info.owner);
        println!("amm_info_data={:?}",amm_info.data);
        let amm_authority_info = next_account_info(account_info_iter)?;
        let amm_open_orders_info = next_account_info(account_info_iter)?;
        if input_account_len == ACCOUNT_LEN + 1 {
            let _amm_target_orders_info = next_account_info(account_info_iter)?;
        }
        let amm_coin_vault_info = next_account_info(account_info_iter)?;
        let amm_pc_vault_info = next_account_info(account_info_iter)?;

        let market_porgram_info = next_account_info(account_info_iter)?;

        let mut amm = AmmInfo::load_mut_checked(&amm_info, program_id)?;
        let enable_orderbook;
        if AmmStatus::from_u64(amm.status).orderbook_permission() {
            enable_orderbook = true;
        } else {
            enable_orderbook = false;
        }
        let market_info = next_account_info(account_info_iter)?;
        let market_bids_info = next_account_info(account_info_iter)?;
        let market_asks_info = next_account_info(account_info_iter)?;
        let market_event_queue_info = next_account_info(account_info_iter)?;
        let market_coin_vault_info = next_account_info(account_info_iter)?;
        let market_pc_vault_info = next_account_info(account_info_iter)?;
        let market_vault_signer = next_account_info(account_info_iter)?;

        let user_source_info = next_account_info(account_info_iter)?;
        let user_destination_info = next_account_info(account_info_iter)?;
        let user_source_owner = next_account_info(account_info_iter)?;


        let spl_token_program_id = token_program_info.key;

        let amm_coin_vault =
            Self::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
        let amm_pc_vault = Self::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;

        let user_source = Self::unpack_token_account(&user_source_info, spl_token_program_id)?;
        let user_destination =
            Self::unpack_token_account(&user_destination_info, spl_token_program_id)?;

        if !AmmStatus::from_u64(amm.status).swap_permission() {
            msg!(&format!("swap_base_in: status {}", amm.status));
            let clock = Clock::get()?;
            if amm.status == AmmStatus::OrderBookOnly.into_u64()
                && (clock.unix_timestamp as u64) >= amm.state_data.orderbook_to_init_time
            {
                amm.status = AmmStatus::Initialized.into_u64();
                msg!("swap_base_in: OrderBook to Initialized");
            } else {
                return Err(AmmError::InvalidStatus.into());
            }
        } else if amm.status == AmmStatus::WaitingTrade.into_u64() {
            let clock = Clock::get()?;
            if (clock.unix_timestamp as u64) < amm.state_data.pool_open_time {
                return Err(AmmError::InvalidStatus.into());
            } else {
                amm.status = AmmStatus::SwapOnly.into_u64();
                msg!("swap_base_in: WaitingTrade to SwapOnly");
            }
        }

        let total_pc_without_take_pnl;
        let total_coin_without_take_pnl;
        let mut bids: Vec<LeafNode> = Vec::new();
        let mut asks: Vec<LeafNode> = Vec::new();
        if enable_orderbook {
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                false,
            )?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            (bids, asks) = Self::get_amm_orders(&open_orders, bids_orders, asks_orders)?;
            (total_pc_without_take_pnl, total_coin_without_take_pnl) =
                Calculator::calc_total_without_take_pnl(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &open_orders,
                    &amm,
                    &market_state,
                    &market_event_queue_info,
                    &amm_open_orders_info,
                )?;
        } else {
            let open_orders = Self::load_orders(amm_open_orders_info)?;
            (total_pc_without_take_pnl, total_coin_without_take_pnl) =
                Calculator::calc_total_without_take_pnl_no_orderbook(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &open_orders,
                    &amm,
                )?;
        }

        let swap_direction;
        if user_source.mint == amm_coin_vault.mint && user_destination.mint == amm_pc_vault.mint {
            swap_direction = SwapDirection::Coin2PC
        } else if user_source.mint == amm_pc_vault.mint
            && user_destination.mint == amm_coin_vault.mint
        {
            swap_direction = SwapDirection::PC2Coin
        } else {
            return Err(AmmError::InvalidUserToken.into());
        }

        let swap_fee = U128::from(swap.amount_in)
            .checked_mul(amm.fees.swap_fee_numerator.into())
            .unwrap()
            .checked_ceil_div(amm.fees.swap_fee_denominator.into())
            .unwrap()
            .0;
        let swap_in_after_deduct_fee = U128::from(swap.amount_in).checked_sub(swap_fee).unwrap();
        let swap_amount_out = Calculator::swap_token_amount_base_in(
            swap_in_after_deduct_fee,
            total_pc_without_take_pnl.into(),
            total_coin_without_take_pnl.into(),
            swap_direction,
        )
            .as_u64();
        match swap_direction {
            SwapDirection::Coin2PC => {
                if swap_amount_out >= total_pc_without_take_pnl {
                    return Err(AmmError::InsufficientFunds.into());
                }

                if enable_orderbook {
                    // coin -> pc, need cancel buy order
                    if !bids.is_empty() {
                        let mut amm_order_ids_vec = Vec::new();
                        let mut order_ids = [0u64; 8];
                        let mut count = 0;
                        // fetch cancel order ids{
                        for order in bids.into_iter() {
                            order_ids[count] = order.client_order_id();
                            count += 1;
                            if count == 8 {
                                amm_order_ids_vec.push(order_ids);
                                order_ids = [0u64; 8];
                                count = 0;
                            }
                        }
                        if count != 0 {
                            amm_order_ids_vec.push(order_ids);
                        }
                        for ids in amm_order_ids_vec.iter() {}
                    }

                    if swap_amount_out > amm_pc_vault.amount {}
                }


                // update state_data data
                amm.state_data.swap_coin_in_amount = amm
                    .state_data
                    .swap_coin_in_amount
                    .checked_add(swap.amount_in.into())
                    .unwrap();
                amm.state_data.swap_pc_out_amount = amm
                    .state_data
                    .swap_pc_out_amount
                    .checked_add(swap_amount_out.into())
                    .unwrap();
                // charge coin as swap fee
                amm.state_data.swap_acc_coin_fee = amm
                    .state_data
                    .swap_acc_coin_fee
                    .checked_add(swap_fee.as_u64())
                    .unwrap();
            }
            SwapDirection::PC2Coin => {
                if swap_amount_out >= total_coin_without_take_pnl {
                    return Err(AmmError::InsufficientFunds.into());
                }

                if enable_orderbook {
                    // pc -> coin, need cancel sell order
                    if !asks.is_empty() {
                        let mut amm_order_ids_vec = Vec::new();
                        let mut order_ids = [0u64; 8];
                        let mut count = 0;
                        // fetch cancel order ids{
                        for order in asks.into_iter() {
                            order_ids[count] = order.client_order_id();
                            count += 1;
                            if count == 8 {
                                amm_order_ids_vec.push(order_ids);
                                order_ids = [0u64; 8];
                                count = 0;
                            }
                        }
                        if count != 0 {
                            amm_order_ids_vec.push(order_ids);
                        }
                        for ids in amm_order_ids_vec.iter() {}
                    }

                    if swap_amount_out > amm_coin_vault.amount {}
                }

                // update state_data data
                amm.state_data.swap_pc_in_amount = amm
                    .state_data
                    .swap_pc_in_amount
                    .checked_add(swap.amount_in.into())
                    .unwrap();
                amm.state_data.swap_coin_out_amount = amm
                    .state_data
                    .swap_coin_out_amount
                    .checked_add(swap_amount_out.into())
                    .unwrap();
                // charge pc as swap fee
                amm.state_data.swap_acc_pc_fee = amm
                    .state_data
                    .swap_acc_pc_fee
                    .checked_add(swap_fee.as_u64())
                    .unwrap();
            }
        };

        Ok(())
    }

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = AmmInstruction::unpack(input)?;
        match instruction {
            AmmInstruction::SwapBaseIn(swap) => {
                Self::process_swap_base_in(program_id, accounts, swap)
            }
            _ => { Ok(()) }
        }
    }
}

pub mod account_parser {
    use crate::raydium::state::AmmInfo;
    use super::*;

    pub struct IdleArgs<'a> {
        pub program_id: &'a Pubkey,
        pub total_coin_without_take_pnl: &'a mut u64,
        pub total_pc_without_take_pnl: &'a mut u64,
        pub amm: &'a mut AmmInfo,
        pub bids: &'a Vec<LeafNode>,
        pub asks: &'a Vec<LeafNode>,
        pub target: &'a mut TargetOrders,
    }

    impl<'a> IdleArgs<'a> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            spl_token_program_id: &'a Pubkey,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo],
            f: impl FnOnce(IdleArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 11 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let &[ref amm_info, ref _market_program_info, ref market_info, ref market_bids_info, ref market_asks_info, ref market_event_queue_info, ref market_authority_info, ref amm_open_orders_info, ref amm_coin_vault_info, ref amm_pc_vault_info, ref amm_target_orders_info] =
                array_ref![accounts, 0, 11];

            let amm_coin_vault =
                Processor::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
            let amm_pc_vault =
                Processor::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;
            let mut target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;

            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                market_authority_info,
                &amm,
                false,
            )?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            let (bids, asks) = Processor::get_amm_orders(&open_orders, bids_orders, asks_orders)?;

            let (mut total_pc_without_take_pnl, mut total_coin_without_take_pnl) =
                Calculator::calc_total_without_take_pnl(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &open_orders,
                    &amm,
                    &market_state,
                    &market_event_queue_info,
                    &amm_open_orders_info,
                )?;

            let args = IdleArgs {
                program_id,
                total_coin_without_take_pnl: &mut total_coin_without_take_pnl,
                total_pc_without_take_pnl: &mut total_pc_without_take_pnl,
                amm,
                bids: &bids,
                asks: &asks,
                target: &mut target,
            };
            f(args)
        }
    }

    pub struct CancelAllOrdersArgs<'a, 'b: 'a> {
        pub program_id: &'a Pubkey,
        pub limit: u16,
        pub market_program_info: &'a AccountInfo<'b>,
        pub market_info: &'a AccountInfo<'b>,
        pub amm_open_orders_info: &'a AccountInfo<'b>,
        pub amm_authority_info: &'a AccountInfo<'b>,
        pub market_event_queue_info: &'a AccountInfo<'b>,
        pub market_coin_vault_info: &'a AccountInfo<'b>,
        pub market_pc_vault_info: &'a AccountInfo<'b>,
        pub market_bids_info: &'a AccountInfo<'b>,
        pub market_asks_info: &'a AccountInfo<'b>,
        pub market_vault_signer: &'a AccountInfo<'b>,
        pub token_program_info: &'a AccountInfo<'b>,
        pub amm_coin_vault_info: &'a AccountInfo<'b>,
        pub amm_pc_vault_info: &'a AccountInfo<'b>,

        pub referrer_pc_account: Option<&'a AccountInfo<'b>>,

        pub amm: &'a mut AmmInfo,
        pub open_orders: &'a OpenOrders,
        pub target: &'a mut TargetOrders,
        pub bids: &'a Vec<LeafNode>,
        pub asks: &'a Vec<LeafNode>,
    }

    impl<'a, 'b: 'a> CancelAllOrdersArgs<'a, 'b> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            limit: u16,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo<'b>],
            referrer_pc_wallet: Option<&'a AccountInfo<'b>>,
            f: impl FnOnce(CancelAllOrdersArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 15 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let &[ref amm_info, ref market_program_info, ref market_info, ref amm_open_orders_info, ref amm_authority_info, ref market_event_queue_info, ref market_coin_vault_info, ref market_pc_vault_info, ref market_bids_info, ref market_asks_info, ref market_vault_signer, ref token_program_info, ref amm_coin_vault_info, ref amm_pc_vault_info, ref amm_target_orders_info] =
                array_ref![accounts, 0, 15];

            let mut target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                true,
            )?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            let (bids, asks) = Processor::get_amm_orders(&open_orders, bids_orders, asks_orders)?;

            let args = CancelAllOrdersArgs {
                program_id,
                limit,
                market_program_info,
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                market_event_queue_info,
                market_coin_vault_info,
                market_pc_vault_info,
                market_bids_info,
                market_asks_info,
                market_vault_signer,
                token_program_info,
                amm_coin_vault_info,
                amm_pc_vault_info,
                referrer_pc_account: referrer_pc_wallet,
                amm,
                open_orders: &open_orders,
                target: &mut target,
                bids: &bids,
                asks: &asks,
            };
            f(args)
        }
    }

    pub struct PlanOrderBookArgs<'a> {
        pub program_id: &'a Pubkey,
        pub limit: u16,
        pub total_coin_without_take_pnl: u64,
        pub total_pc_without_take_pnl: u64,
        pub amm: &'a mut AmmInfo,
        pub target: &'a mut TargetOrders,
    }

    impl<'a> PlanOrderBookArgs<'a> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            spl_token_program_id: &'a Pubkey,
            limit: u16,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo],
            f: impl FnOnce(PlanOrderBookArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 8 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let &[ref amm_info, ref market_info, ref market_event_queue_info, ref amm_authority_info, ref amm_open_orders_info, ref amm_coin_vault_info, ref amm_pc_vault_info, ref amm_target_orders_info] =
                array_ref![accounts, 0, 8];

            let amm_coin_vault =
                Processor::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
            let amm_pc_vault =
                Processor::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;
            let mut target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                false,
            )?;
            let (total_pc_without_take_pnl, total_coin_without_take_pnl) =
                Calculator::calc_total_without_take_pnl(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &open_orders,
                    &amm,
                    &market_state,
                    &market_event_queue_info,
                    &amm_open_orders_info,
                )?;

            let args = PlanOrderBookArgs {
                program_id,
                limit,
                total_coin_without_take_pnl,
                total_pc_without_take_pnl,
                amm,
                target: &mut target,
            };
            f(args)
        }
    }

    pub struct PlaceOrdersArgs<'a, 'b: 'a> {
        pub program_id: &'a Pubkey,
        pub limit: u16,

        pub amm_authority_info: &'a AccountInfo<'b>,
        pub amm_open_orders_info: &'a AccountInfo<'b>,
        pub market_program_info: &'a AccountInfo<'b>,
        pub market_info: &'a AccountInfo<'b>,
        pub market_request_queue_info: &'a AccountInfo<'b>,
        pub amm_coin_vault_info: &'a AccountInfo<'b>,
        pub amm_pc_vault_info: &'a AccountInfo<'b>,
        pub market_coin_vault_info: &'a AccountInfo<'b>,
        pub market_pc_vault_info: &'a AccountInfo<'b>,
        pub token_program_info: &'a AccountInfo<'b>,
        pub rent_info: &'a AccountInfo<'b>,

        pub market_event_queue_info: &'a AccountInfo<'b>,
        pub market_bids_info: &'a AccountInfo<'b>,
        pub market_asks_info: &'a AccountInfo<'b>,

        pub srm_token_account: Option<&'a AccountInfo<'b>>,

        pub amm: &'a mut AmmInfo,
        pub open_orders: &'a OpenOrders,
        pub bids: &'a Vec<LeafNode>,
        pub asks: &'a Vec<LeafNode>,
        pub target: &'a mut TargetOrders,

        pub total_coin_without_take_pnl: u64,
        pub total_pc_without_take_pnl: u64,
        pub coin_vault_amount: u64,
        pub pc_vault_amount: u64,
    }

    impl<'a, 'b: 'a> PlaceOrdersArgs<'a, 'b> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            spl_token_program_id: &'a Pubkey,
            limit: u16,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo<'b>],
            srm_token_account: Option<&'a AccountInfo<'b>>,
            f: impl FnOnce(PlaceOrdersArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 17 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let accounts = array_ref![accounts, 0, 17];
            let (new_orders_accounts, data_accounts) = array_refs![accounts, 15, 2];
            let &[ref amm_info, ref amm_authority_info, ref amm_open_orders_info, ref market_program_info, ref market_info, ref market_request_queue_info, ref amm_coin_vault_info, ref amm_pc_vault_info, ref market_coin_vault_info, ref market_pc_vault_info, ref token_program_info, ref rent_info, ref market_event_queue_info, ref market_bids_info, ref market_asks_info] =
                array_ref![new_orders_accounts, 0, 15];
            let &[ref amm_target_orders_info, ref _clock_info] = array_ref![data_accounts, 0, 2];

            let amm_coin_vault =
                Processor::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
            let amm_pc_vault =
                Processor::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;
            let mut target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                false,
            )?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            let (bids, asks) = Processor::get_amm_orders(&open_orders, bids_orders, asks_orders)?;
            let (total_pc_without_take_pnl, total_coin_without_take_pnl) =
                Calculator::calc_total_without_take_pnl(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &open_orders,
                    &amm,
                    &market_state,
                    &market_event_queue_info,
                    &amm_open_orders_info,
                )?;

            let args = PlaceOrdersArgs {
                program_id,
                limit,

                amm_authority_info,
                amm_open_orders_info,
                market_program_info,
                market_info,
                market_request_queue_info,
                amm_coin_vault_info,
                amm_pc_vault_info,
                market_coin_vault_info,
                market_pc_vault_info,
                token_program_info,
                rent_info,

                market_event_queue_info,
                market_bids_info,
                market_asks_info,
                srm_token_account,

                amm,
                open_orders: &open_orders,
                bids: &bids,
                asks: &asks,
                target: &mut target,

                total_pc_without_take_pnl,
                total_coin_without_take_pnl,

                coin_vault_amount: amm_coin_vault.amount,
                pc_vault_amount: amm_pc_vault.amount,
            };
            f(args)
        }
    }

    pub struct PurgeOrderArgs<'a, 'b: 'a> {
        pub program_id: &'a Pubkey,
        pub limit: u16,
        pub market_program_info: &'a AccountInfo<'b>,
        pub market_info: &'a AccountInfo<'b>,
        pub amm_open_orders_info: &'a AccountInfo<'b>,
        pub amm_authority_info: &'a AccountInfo<'b>,
        pub market_event_queue_info: &'a AccountInfo<'b>,
        pub market_bids_info: &'a AccountInfo<'b>,
        pub market_asks_info: &'a AccountInfo<'b>,
        pub amm: &'a mut AmmInfo,
        pub target: &'a TargetOrders,
        pub bids: &'a Vec<LeafNode>,
        pub asks: &'a Vec<LeafNode>,
    }

    impl<'a, 'b: 'a> PurgeOrderArgs<'a, 'b> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            _spl_token_program_id: &'a Pubkey,
            limit: u16,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo<'b>],
            f: impl FnOnce(PurgeOrderArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 9 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let &[ref amm_info, ref market_program_info, ref market_info, ref amm_open_orders_info, ref amm_authority_info, ref market_event_queue_info, ref market_bids_info, ref market_asks_info, ref amm_target_orders_info] =
                array_ref![accounts, 0, 9];

            let target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                true,
            )?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            let (bids, asks) = Processor::get_amm_orders(&open_orders, bids_orders, asks_orders)?;

            let args = PurgeOrderArgs {
                program_id,
                limit,
                market_program_info,
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                market_event_queue_info,
                market_bids_info,
                market_asks_info,
                amm,
                target: &target,
                bids: &bids,
                asks: &asks,
            };
            f(args)
        }
    }

    pub struct CancelOrderArgs<'a, 'b: 'a> {
        pub program_id: &'a Pubkey,
        pub limit: u16,
        pub market_program_info: &'a AccountInfo<'b>,
        pub market_info: &'a AccountInfo<'b>,
        pub amm_open_orders_info: &'a AccountInfo<'b>,
        pub amm_authority_info: &'a AccountInfo<'b>,
        pub market_event_queue_info: &'a AccountInfo<'b>,
        pub market_bids_info: &'a AccountInfo<'b>,
        pub market_asks_info: &'a AccountInfo<'b>,
        pub amm: &'a mut AmmInfo,
        pub open_orders: &'a OpenOrders,
        pub target: &'a mut TargetOrders,
        pub bids: &'a Vec<LeafNode>,
        pub asks: &'a Vec<LeafNode>,

        pub coin_amount: u64,
        pub pc_amount: u64,
    }

    impl<'a, 'b: 'a> CancelOrderArgs<'a, 'b> {
        pub fn with_parsed_args(
            program_id: &'a Pubkey,
            spl_token_program_id: &'a Pubkey,
            limit: u16,
            amm: &mut AmmInfo,
            accounts: &'a [AccountInfo<'b>],
            f: impl FnOnce(CancelOrderArgs) -> ProgramResult,
        ) -> ProgramResult {
            if accounts.len() != 11 {
                return Err(AmmError::WrongAccountsNumber.into());
            }
            let &[ref amm_info, ref market_program_info, ref market_info, ref amm_open_orders_info, ref amm_authority_info, ref market_event_queue_info, ref market_bids_info, ref market_asks_info, ref amm_coin_vault_info, ref amm_pc_vault_info, ref amm_target_orders_info] =
                array_ref![accounts, 0, 11];

            let amm_coin_vault =
                Processor::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
            let amm_pc_vault =
                Processor::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;
            let mut target =
                TargetOrders::load_mut_checked(&amm_target_orders_info, program_id, amm_info.key)?;
            let (market_state, open_orders) = Processor::load_serum_market_order(
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                &amm,
                true,
            )?;
            // let (myorders, _max_bid, _min_ask) = Processor::get_amm_orders_and_dex_best_price(&market_state, &open_orders, bids_acc, asks_acc)?;
            let bids_orders = market_state.load_bids_mut(&market_bids_info)?;
            let asks_orders = market_state.load_asks_mut(&market_asks_info)?;
            let (bids, asks) = Processor::get_amm_orders(&open_orders, bids_orders, asks_orders)?;

            let args = CancelOrderArgs {
                program_id,
                limit,
                market_program_info,
                market_info,
                amm_open_orders_info,
                amm_authority_info,
                market_event_queue_info,
                market_bids_info,
                market_asks_info,
                amm,
                open_orders: &open_orders,
                target: &mut target,
                bids: &bids,
                asks: &asks,
                coin_amount: amm_coin_vault.amount,
                pc_amount: amm_pc_vault.amount,
            };
            f(args)
        }
    }
}

