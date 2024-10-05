use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("AUCTION1111111111111111111111111111111111111");

#[program]
pub mod advanced_auction {
    use super::*;

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        start_price: u64,
        reserve_price: u64,
        buy_now_price: Option<u64>,
        duration: i64,
        platform_fee: u64,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let current_time = Clock::get()?.unix_timestamp;

        auction.seller = ctx.accounts.seller.key();
        auction.nft_token_account = ctx.accounts.nft_token_account.key();
        auction.start_price = start_price;
        auction.reserve_price = reserve_price;
        auction.buy_now_price = buy_now_price;
        auction.highest_bid = 0;
        auction.highest_bidder = Pubkey::default();
        auction.end_time = current_time + duration;
        auction.is_active = true;
        auction.platform_fee = platform_fee;

        // Transfer the NFT to the auction vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.nft_token_account.to_account_info(),
            to: ctx.accounts.auction_vault.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn place_bid(ctx: Context<PlaceBid>, amount: u64) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let current_time = Clock::get()?.unix_timestamp;

        require!(auction.is_active, ErrorCode::AuctionNotActive);
        require!(current_time < auction.end_time, ErrorCode::AuctionEnded);
        require!(amount > auction.highest_bid, ErrorCode::BidTooLow);

        // Refund the previous highest bidder
        if auction.highest_bid > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.auction_vault.to_account_info(),
                to: ctx.accounts.highest_bidder_token_account.to_account_info(),
                authority: ctx.accounts.auction_authority.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, auction.highest_bid)?;
        }

        // Accept the new bid
        auction.highest_bid = amount;
        auction.highest_bidder = ctx.accounts.bidder.key();

        // Handle Buy Now option
        if let Some(buy_now_price) = auction.buy_now_price {
            if amount >= buy_now_price {
                auction.end_time = current_time; // End the auction immediately
            }
        }

        Ok(())
    }

    pub fn withdraw_bid(ctx: Context<WithdrawBid>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let bidder_key = ctx.accounts.bidder.key();

        require!(auction.highest_bidder == bidder_key, ErrorCode::NotHighestBidder);

        let amount = auction.highest_bid;

        // Transfer back the highest bid to the bidder
        let cpi_accounts = Transfer {
            from: ctx.accounts.auction_vault.to_account_info(),
            to: ctx.accounts.bidder_token_account.to_account_info(),
            authority: ctx.accounts.auction_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Reset highest bidder and bid
        auction.highest_bidder = Pubkey::default();
        auction.highest_bid = 0;

        Ok(())
    }

    pub fn cancel_auction(ctx: Context<CancelAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        require!(auction.highest_bid == 0, ErrorCode::BidsAlreadyPlaced);
        
        // Transfer the NFT back to the seller
        let cpi_accounts = Transfer {
            from: ctx.accounts.auction_vault.to_account_info(),
            to: ctx.accounts.seller_nft_account.to_account_info(),
            authority: ctx.accounts.auction_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        auction.is_active = false;

        Ok(())
    }

    pub fn end_auction(ctx: Context<EndAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let current_time = Clock::get()?.unix_timestamp;

        require!(auction.is_active, ErrorCode::AuctionNotActive);
        require!(current_time >= auction.end_time, ErrorCode::AuctionNotEnded);

        // Ensure reserve price is met
        if auction.highest_bid < auction.reserve_price {
            return Err(error!(ErrorCode::ReservePriceNotMet));
        }

        auction.is_active = false;

        // Transfer platform fee to platform account
        let platform_fee = (auction.highest_bid * auction.platform_fee) / 100;
        let seller_amount = auction.highest_bid - platform_fee;

        // Transfer NFT to highest bidder
        let cpi_accounts = Transfer {
            from: ctx.accounts.auction_vault.to_account_info(),
            to: ctx.accounts.highest_bidder_nft_account.to_account_info(),
            authority: ctx.accounts.auction_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        // Transfer funds to seller
        let cpi_accounts = Transfer {
            from: ctx.accounts.auction_vault.to_account_info(),
            to: ctx.accounts.seller_token_account.to_account_info(),
            authority: ctx.accounts.auction_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, seller_amount)?;

        // Transfer platform fee to the platform
        let cpi_accounts = Transfer {
            from: ctx.accounts.auction_vault.to_account_info(),
            to: ctx.accounts.platform_account.to_account_info(),
            authority: ctx.accounts.auction_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, platform_fee)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateAuction<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction_vault: Account<'info, TokenAccount>,
    #[account(init, payer = seller, space = 8 + 200)]
    pub auction: Account<'info, Auction>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(mut)]
    pub highest_bidder_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(seeds = [b"auction-authority"], bump)]
    pub auction_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(mut)]
    pub bidder_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(seeds = [b"auction-authority"], bump)]
    pub auction_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelAuction<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub seller_nft_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(seeds = [b"auction-authority"], bump)]
    pub auction_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub highest_bidder_nft_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub platform_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(seeds = [b"auction-authority"], bump)]
    pub auction_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Auction {
    pub seller: Pubkey,
    pub nft_token_account: Pubkey,
    pub start_price: u64,
    pub reserve_price: u64,
    pub highest_bid: u64,
    pub highest_bidder: Pubkey,
    pub end_time: i64,
    pub is_active: bool,
    pub platform_fee: u64,
    pub buy_now_price: Option<u64>,
}

#[error]
pub enum ErrorCode {
    #[msg("Auction is not active")]
    AuctionNotActive,
    #[msg("Auction has already ended")]
    AuctionEnded,
    #[msg("Bid is too low")]
    BidTooLow,
    #[msg("Reserve price was not met")]
    ReservePriceNotMet,
    #[msg("Bids have already been placed, cannot cancel")]
    BidsAlreadyPlaced,
    #[msg("Only the highest bidder can withdraw their bid")]
    NotHighestBidder,
    #[msg("Auction has not ended yet")]
    AuctionNotEnded,
}