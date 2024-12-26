pub mod client;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::cmp::{max, min};

pub const MILLI_BIPS_SCALE: u128 = 10_000_000;
pub const MAX_EDGE_MULTIPLIER_MILLIS: u64 = 100 * 1000; // 100x

#[derive(Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct MarketConfig {
    pub enabled: u8,
    pub _padding_0: [u8; 7],
    pub size_edge_spline: Spline,
    pub time_edge_spline: Spline,
    pub retreat_milli_bips: u64,
    pub retreat_quote_amount: u64,
    pub max_retreat_up_milli_bips: u64,
    pub max_retreat_down_milli_bips: u64,
    pub _padding_1: [u64; 16],
}

#[derive(Default, Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct MarketPrice {
    pub price_decimals: i64,
    pub price_quote_atoms_per_base_atom: u64,
    pub price_updated_slot: u64,
    pub price_updated_ms: u64,
    pub volatility_milli_scale: u64,
    pub price_last_valid_slot: u64,
    pub _padding_0: [u64; 15],
}

impl MarketPrice {
    pub fn swap_fair_price_conversion(
        &self,
        amount_in: u64,
        is_quote_to_base: bool,
        price_quote_atoms_per_base_atom_opt: Option<u64>,
    ) -> Option<u64> {
        let price_quote_atoms_per_base_atom =
            price_quote_atoms_per_base_atom_opt.unwrap_or(self.price_quote_atoms_per_base_atom);

        let price_decimal_scale = u64::pow(10, self.price_decimals.unsigned_abs() as u32) as u128;

        if is_quote_to_base {
            let mut base_atoms_out = if self.price_decimals < 0 {
                (amount_in as u128).checked_mul(price_decimal_scale)?
            } else {
                (amount_in as u128).checked_div(price_decimal_scale)?
            };

            // do div here after price_decimal scale to maintain precision (possible mult above before div here)
            base_atoms_out = base_atoms_out.checked_div(price_quote_atoms_per_base_atom.into())?;

            return base_atoms_out.try_into().ok();
        } else {
            // do mult here before price_decimal scale to maintain precision (mult here before possible div below)
            let mut quote_atoms_out =
                (amount_in as u128).checked_mul(price_quote_atoms_per_base_atom.into())?;

            quote_atoms_out = if self.price_decimals > 0 {
                quote_atoms_out.checked_mul(price_decimal_scale)?
            } else {
                quote_atoms_out.checked_div(price_decimal_scale)?
            };

            return quote_atoms_out.try_into().ok();
        }
    }
}

#[derive(Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct MarketAccount {
    pub bump: u8,
    pub _padding_0: [u8; 7],
    pub market_config: MarketConfig,
    pub market_price: MarketPrice,
    pub _padding_1: [Pubkey; 64],
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint_decimals: u32,
    pub quote_mint_decimals: u32,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
}

impl MarketAccount {
    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        bytemuck::try_from_bytes::<MarketAccount>(bytes)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        bytemuck::try_from_bytes_mut::<MarketAccount>(bytes)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn get_fair_with_inventory_retreat(
        &self,
        base_vault_amount: u64,
        quote_vault_amount: u64,
    ) -> Result<u64, ProgramError> {
        let base_vault_equiv_in_quote_atoms =
            self.market_price
                .swap_fair_price_conversion(base_vault_amount, false, None)
                .ok_or(ProgramError::InvalidAccountData)? as i64;

        // Figure out how imbalanced the inventory is
        let extra_quote_atoms = (quote_vault_amount as i64 - base_vault_equiv_in_quote_atoms) / 2;

        // Calculate the retreat amount (milli bips)
        let mut milli_bips_change = extra_quote_atoms
            .checked_mul(self.market_config.retreat_milli_bips as i64)
            .and_then(|x| x.checked_div(self.market_config.retreat_quote_amount as i64))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Limit the retreat to the max retreat up and down
        milli_bips_change = milli_bips_change.clamp(
            -(self.market_config.max_retreat_down_milli_bips as i64),
            self.market_config.max_retreat_up_milli_bips as i64,
        );

        // Turn milli bips into a proportion close to 1, for multiplying vs. the oracle price
        let milli_bips_numerator = (MILLI_BIPS_SCALE as i128)
            .checked_add(milli_bips_change as i128)
            .ok_or(ProgramError::ArithmeticOverflow)? as u128;

        let new_price_quote_atoms_per_base_atom =
            (self.market_price.price_quote_atoms_per_base_atom as u128)
                .checked_mul(milli_bips_numerator)
                .and_then(|x| x.checked_div(MILLI_BIPS_SCALE))
                .ok_or(ProgramError::ArithmeticOverflow)?;

        let new_price_quote_atoms_per_base_atom_u64 =
            if new_price_quote_atoms_per_base_atom > u64::MAX as u128 {
                return Err(ProgramError::ArithmeticOverflow);
            } else {
                new_price_quote_atoms_per_base_atom as u64
            };

        Ok(new_price_quote_atoms_per_base_atom_u64)
    }

    pub fn swap_amount_out(
        &self,
        amount_in: u64,
        is_quote_to_base: bool,
        slot: u64,
        base_vault_amount: u64,
        quote_vault_amount: u64,
    ) -> Result<u64, ProgramError> {
        // Get the fair price with inventory retreat
        let fair_with_retreat =
            self.get_fair_with_inventory_retreat(base_vault_amount, quote_vault_amount)?;

        // Calculate the swap (with retreat, but not yet edge)
        let fair_amount_out = self
            .market_price
            .swap_fair_price_conversion(amount_in, is_quote_to_base, Some(fair_with_retreat))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Figure out the swap size for the size edge calc
        let effective_quote_amount = if is_quote_to_base {
            amount_in
        } else {
            fair_amount_out
        };

        // Base edge, from spline, based on size (no min)
        let size_edge_milli = self
            .market_config
            .size_edge_spline
            .eval(effective_quote_amount);

        // Time edge, from spline, based on how stale the oracle price is. Min is 1000/1000 = 1 since this is a positive multiplier.
        let mut time_edge_milli_mult = max(
            1000,
            self.market_config
                .time_edge_spline
                .eval(slot.saturating_sub(self.market_price.price_updated_slot)),
        );
        time_edge_milli_mult = min(time_edge_milli_mult, MAX_EDGE_MULTIPLIER_MILLIS); // sane max value of 100x to prevent overflow in edge_milli_bips calc

        // Volatility edge, from vol param. Min is 1000/1000 = 1 since this is a positive multiplier.
        let mut vol_edge_milli_mult = max(1000, self.market_price.volatility_milli_scale);
        vol_edge_milli_mult = min(vol_edge_milli_mult, MAX_EDGE_MULTIPLIER_MILLIS); // sane max value of 100x to prevent overflow in edge_milli_bips calc

        // Combine the edges, into a single milli bips value
        let edge_milli_bips = (size_edge_milli as u128)
            .checked_mul(time_edge_milli_mult as u128)
            .and_then(|x| x.checked_mul(vol_edge_milli_mult as u128))
            .and_then(|x| x.checked_div(1000 * 1000 * 1000))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Turn the milli bips into a proportion close to 1, for multiplying vs. the amount out
        let amount_out_milli_bips = MILLI_BIPS_SCALE
            .checked_sub(edge_milli_bips)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let amount_out = (fair_amount_out as u128)
            .checked_mul(amount_out_milli_bips)
            .and_then(|x| x.checked_div(MILLI_BIPS_SCALE))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let amount_out_u64 = if amount_out > u64::MAX as u128 {
            return Err(ProgramError::ArithmeticOverflow);
        } else {
            amount_out as u64
        };

        Ok(amount_out_u64)
    }
}

#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct Spline {
    pub x: [u64; 8],
    pub y: [u64; 8],
    pub len: u64,
}

impl Spline {
    pub fn is_valid(&self, max_value: u64) -> Result<()> {
        if self.len == 0 || self.len > 8 {
            return Err(anyhow::anyhow!(
                "Spline length must be greater than 0 and 8 or less"
            ));
        }

        if self.x[0] != 0 {
            return Err(anyhow::anyhow!("Spline x[0] must be 0"));
        }

        for i in 0..self.len as usize - 1 {
            if self.x[i + 1] <= self.x[i] || self.y[i + 1] <= self.y[i] {
                return Err(anyhow::anyhow!(
                    "Points must be monotonically increasing in both x and y"
                ));
            }
            if self.y[i] > max_value {
                return Err(anyhow::anyhow!(
                    "Spline y[{}] must be less than max_value",
                    i
                ));
            }
        }

        Ok(())
    }

    pub fn eval(&self, x: u64) -> u64 {
        if self.len == 0 {
            return 0;
        }

        // Find the two points to interpolate between
        for i in 0..self.len as usize - 1 {
            let x1 = self.x[i];
            let y1 = self.y[i];
            let x2 = self.x[i + 1];
            let y2 = self.y[i + 1];

            // If x is less than or equal to the first point, return y1
            if x <= x1 {
                return y1;
            }

            if x < x2 {
                // Perform linear interpolation using integer arithmetic
                let dx = x2 - x1;
                let dy = y2 - y1;
                let offset = x - x1;

                // Use integer division with rounding
                return y1 + (dy * offset + dx / 2) / dx;
            }
        }

        // If we've reached here, x is beyond the last point, so return the last y-value
        self.y[self.len as usize - 1]
    }
}


