use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("VESTING1111111111111111111111111111111111111");

#[program]
pub mod token_vesting {
    use super::*;

    pub fn initialize_vesting(
        ctx: Context<InitializeVesting>,
        cliff_duration: i64,
        vesting_duration: i64,
        total_amount: u64,
    ) -> Result<()> {
        let vesting_account = &mut ctx.accounts.vesting_account;
        let current_time = Clock::get()?.unix_timestamp;

        vesting_account.beneficiary = ctx.accounts.beneficiary.key();
        vesting_account.total_amount = total_amount;
        vesting_account.claimed_amount = 0;
        vesting_account.start_time = current_time;
        vesting_account.cliff_time = current_time + cliff_duration;
        vesting_account.vesting_end_time = current_time + vesting_duration;

        // Transfer tokens to the vesting vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.admin_token_account.to_account_info(),
            to: ctx.accounts.vesting_vault.to_account_info(),
            authority: ctx.accounts.admin.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, total_amount)?;

        Ok(())
    }

    pub fn claim_tokens(ctx: Context<ClaimTokens>) -> Result<()> {
        let vesting_account = &mut ctx.accounts.vesting_account;
        let current_time = Clock::get()?.unix_timestamp;
        let beneficiary_key = ctx.accounts.beneficiary.key();

        require!(vesting_account.beneficiary == beneficiary_key, ErrorCode::Unauthorized);
        require!(current_time >= vesting_account.cliff_time, ErrorCode::CliffNotReached);

        let vested_amount = get_vested_amount(vesting_account, current_time)?;
        let claimable_amount = vested_amount - vesting_account.claimed_amount;
        require!(claimable_amount > 0, ErrorCode::NoTokensAvailable);

        // Transfer claimable tokens to beneficiary
        let cpi_accounts = Transfer {
            from: ctx.accounts.vesting_vault.to_account_info(),
            to: ctx.accounts.beneficiary_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, claimable_amount)?;

        vesting_account.claimed_amount += claimable_amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVesting<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut)]
    pub admin_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vesting_vault: Account<'info, TokenAccount>,
    #[account(init, payer = admin, space = 8 + 200)]
    pub vesting_account: Account<'info, VestingAccount>,
    pub beneficiary: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimTokens<'info> {
    #[account(mut)]
    pub beneficiary: Signer<'info>,
    #[account(mut)]
    pub vesting_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub beneficiary_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vesting_account: Account<'info, VestingAccount>,
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct VestingAccount {
    pub beneficiary: Pubkey,
    pub total_amount: u64,
    pub claimed_amount: u64,
    pub start_time: i64,
    pub cliff_time: i64,
    pub vesting_end_time: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized access.")]
    Unauthorized,
    #[msg("Cliff period has not been reached.")]
    CliffNotReached,
    #[msg("No tokens available for claim.")]
    NoTokensAvailable,
}

fn get_vested_amount(vesting_account: &VestingAccount, current_time: i64) -> Result<u64> {
    if current_time >= vesting_account.vesting_end_time {
        Ok(vesting_account.total_amount)
    } else {
        let time_elapsed = current_time - vesting_account.cliff_time;
        let vesting_period = vesting_account.vesting_end_time - vesting_account.cliff_time;
        let vested_percentage = (time_elapsed as u128 * 1_000_000) / (vesting_period as u128);
        Ok(((vesting_account.total_amount as u128 * vested_percentage) / 1_000_000) as u64)
    }
}