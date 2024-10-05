use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWxqSWdixgW8Hd4CPj1Ntb5uZ6bX");

const UNLOCK_PERIOD: i64 = 21 * 24 * 60 * 60; // 21 days in seconds
const AIRDROP_AMOUNT: u64 = 1000 * 1_000_000; // Example: 1000 tokens (assuming 6 decimal places)

#[program]
pub mod staking_airdrop {
    use super::*;

    pub fn claim_airdrop(ctx: Context<ClaimAirdrop>) -> Result<()> {
        let user_state = &mut ctx.accounts.user_state;
        require!(user_state.has_claimed == false, ErrorCode::AirdropAlreadyClaimed);

        let cpi_accounts = Transfer {
            from: ctx.accounts.airdrop_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, AIRDROP_AMOUNT)?;

        user_state.has_claimed = true;
        Ok(())
    }

    pub fn stake_tokens(ctx: Context<StakeTokens>, amount: u64) -> Result<()> {
        let user_state = &mut ctx.accounts.user_state;
        let current_time = Clock::get()?.unix_timestamp;

        // Transfer tokens from user's token account to staking vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;

        user_state.staked_amount += amount;
        user_state.staked_time = current_time;

        Ok(())
    }

    pub fn unstake_tokens(ctx: Context<UnstakeTokens>, amount: u64) -> Result<()> {
        let user_state = &mut ctx.accounts.user_state;
        let current_time = Clock::get()?.unix_timestamp;

        // Check if the unlocking period has passed
        require!(
            current_time >= user_state.staked_time + UNLOCK_PERIOD,
            ErrorCode::TokensLocked
        );
        require!(user_state.staked_amount >= amount, ErrorCode::InsufficientStakedAmount);

        // Transfer tokens back to user's token account
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, amount)?;

        user_state.staked_amount -= amount;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ClaimAirdrop<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub airdrop_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: AccountInfo<'info>,
    #[account(init_if_needed, payer = user, space = 8 + 40, seeds = [user.key().as_ref()], bump)]
    pub user_state: Account<'info, UserState>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: AccountInfo<'info>,
    #[account(mut, seeds = [user.key().as_ref()], bump)]
    pub user_state: Account<'info, UserState>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnstakeTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: AccountInfo<'info>,
    #[account(mut, seeds = [user.key().as_ref()], bump)]
    pub user_state: Account<'info, UserState>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct UserState {
    pub has_claimed: bool,
    pub staked_amount: u64,
    pub staked_time: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("User has already claimed the airdrop.")]
    AirdropAlreadyClaimed,
    #[msg("Tokens are still locked.")]
    TokensLocked,
    #[msg("Insufficient staked amount.")]
    InsufficientStakedAmount,
}