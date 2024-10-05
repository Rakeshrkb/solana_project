use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use anchor_lang::solana_program::clock::Clock;

declare_id!("YourProgramIdHere");

#[program]
mod farming {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, token_mint: Pubkey) -> Result<()> {
        let user = &mut ctx.accounts.user;
        user.owner = ctx.accounts.owner.key();
        user.claim_interval = 30 * 24 * 60 * 60; // 30 days in seconds
        user.token_mint = token_mint;
        Ok(())
    }

    pub fn deposit_tokens(ctx: Context<DepositTokens>, amount: u64, reward_amount: u64) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let clock = Clock::get()?;
        require!(amount > 0, CustomError::InvalidAmount);
        require!(reward_amount <= amount / 2, CustomError::InvalidRewardAmount);

        let total_amount = amount + reward_amount;
        user_info.krpza_deposited_amount += total_amount;
        
        if user_info.last_deposit_time == 0 {
            user_info.last_deposit_time = clock.unix_timestamp;
            user_info.next_claim_time = clock.unix_timestamp + user_info.claim_interval as i64;
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        emit!(Deposit {
            user: *ctx.accounts.user.key,
            amount: total_amount,
        });
        Ok(())
    }

    pub fn claim_monthly(ctx: Context<ClaimMonthly>, months: u64, reduce_deposit_amount: u64) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let clock = Clock::get()?;
        require!(months > 0, CustomError::InvalidMonthCount);
        require!(user_info.krpza_deposited_amount >= reduce_deposit_amount, CustomError::InsufficientBalance);
        require!(clock.unix_timestamp >= user_info.next_claim_time, CustomError::ClaimIntervalNotPassed);
        
        // Check if correct number of months have passed
        let claim_time = user_info.next_claim_time + ((months - 1) as i64 * user_info.claim_interval as i64);
        require!(clock.unix_timestamp >= claim_time, CustomError::InvalidClaimTime);

        user_info.krpza_deposited_amount -= reduce_deposit_amount;
        user_info.next_claim_time = clock.unix_timestamp + (user_info.claim_interval as i64 * months as i64);
        user_info.month_count += months;
        emit!(Claim {
            user: *ctx.accounts.user.key,
            amount: reduce_deposit_amount,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + std::mem::size_of::<UserInfo>(),
    )]
    pub user: Account<'info, UserInfo>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DepositTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_info: Account<'info, UserInfo>,
    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimMonthly<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_info: Account<'info, UserInfo>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct UserInfo {
    pub owner: Pubkey,
    pub krpza_deposited_amount: u64,
    pub last_deposit_time: i64,
    pub next_claim_time: i64,
    pub claim_interval: u64,
    pub month_count: u64,
    pub token_mint: Pubkey,
}

#[error_code]
pub enum CustomError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Reward amount must be less than half of deposit amount")]
    InvalidRewardAmount,
    #[msg("Claim interval has not passed yet")]
    ClaimIntervalNotPassed,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Month count must be greater than 0")]
    InvalidMonthCount,
    #[msg("Invalid claim time")]
    InvalidClaimTime,
}

#[event]
pub struct Deposit {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Claim {
    pub user: Pubkey,
    pub amount: u64,
}