use anchor_lang::prelude::*;

// Define a placeholder program ID for initial testing
declare_id!("9YQK5crT1uqpddaNKGBgmey4NQnkcSYFmHNe8E1zL1V2");

#[program]
pub mod my_solana_project {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let base_account = &mut ctx.accounts.base_account;
        base_account.counter = 0;
        msg!("Initialized counter to 0!");
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let base_account = &mut ctx.accounts.base_account;
        base_account.counter += 1;
        msg!("Counter increased to {}!", base_account.counter);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 4)] // 8 bytes for discriminator + 4 bytes for the counter
    pub base_account: Account<'info, BaseAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    pub base_account: Account<'info, BaseAccount>,
}

#[account]
pub struct BaseAccount {
    pub counter: u32,
}
