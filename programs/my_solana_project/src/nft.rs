[7:34 pm, 5/10/2024] Rakesh Kumar Barik: pub struct Deposit {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Claim {
    pub user: Pubkey,
    pub amount: u64,
}
[7:45 pm, 5/10/2024] Rakesh Kumar Barik: use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token::mint_to;
use anchor_spl::token::{MintTo, Token, Transfer, TransferFrom, TokenAccount};
use mpl_token_metadata::instruction::create_metadata_accounts_v2;

declare_id!("9FKLho9AUYScrrKgJbG1mExt5nSgEfk1CNEbR8qBwKTZ");

#[program]
pub mod nft_minting_and_marketplace {
    use super::*;

    pub fn nft_format(
        ctx: Context<MintNFT>,
        creator_key: Pubkey,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        msg!("Minting NFT:");
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        let result = mint_to(cpi_ctx, 1);
        if let Err(_) = result {
            return Err(error!(ErrorCode::MintFailed));
        }
        msg!("NFT has been minted!");
        msg!("Metadata account is being created:");
        let accounts = vec![
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ];
        let creators = vec![
            mpl_token_metadata::state::Creator {
                address: creator_key,
                verified: false,
                share: 100,
            },
            mpl_token_metadata::state::Creator {
                address: ctx.accounts.mint_authority.key(),
                verified: false,
                share: 0,
            },
        ];
        let result = invoke(
            &create_metadata_accounts_v2(
                ctx.accounts.token_metadata_program.key(),
                ctx.accounts.metadata.key(),
                ctx.accounts.mint.key(),
                ctx.accounts.mint_authority.key(),
                ctx.accounts.payer.key(),
                ctx.accounts.payer.key(),
                name,
                symbol,
                uri,
                Some(creators),
                1,
                true,
                false,
                None,
                None,
            ),
            &accounts,
        );
        if let Err(_) = result {
            return Err(error!(ErrorCode::MetadataCreateFailed));
        }
        msg!("Metadata account has been created");
        Ok(())
    }

    pub fn list_nft(ctx: Context<ListNFT>, price: u64) -> Result<()> {
        let nft_listing = &mut ctx.accounts.nft_listing;
        nft_listing.seller = *ctx.accounts.seller.key;
        nft_listing.mint = *ctx.accounts.mint.key;
        nft_listing.price = price;
        nft_listing.is_listed = true;

        msg!("NFT listed for sale at price: {}", price);
        emit!(NftListed {
            seller: *ctx.accounts.seller.key,
            mint: *ctx.accounts.mint.key,
            price,
        });

        Ok(())
    }

    pub fn purchase_nft(ctx: Context<PurchaseNFT>) -> Result<()> {
        let nft_listing = &mut ctx.accounts.nft_listing;
        let price = nft_listing.price;

        require!(nft_listing.is_listed, ErrorCode::NFTNotListed);
        require!(ctx.accounts.buyer.to_account_info().lamports() >= price, ErrorCode::InsufficientFunds);

        // Transfer funds from buyer to seller
        **ctx.accounts.buyer.to_account_info().try_borrow_mut_lamports()? -= price;
        **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? += price;

        // Transfer the NFT to the buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token::transfer(cpi_ctx, 1)?;

        // Mark the NFT as sold
        nft_listing.is_listed = false;
        msg!("NFT purchased successfully!");

        emit!(NftSold {
            seller: *ctx.accounts.seller.key,
            buyer: *ctx.accounts.buyer.key,
            mint: *ctx.accounts.mint.key,
            price,
        });

        Ok(())
    }

    pub fn delist_nft(ctx: Context<DelistNFT>) -> Result<()> {
        let nft_listing = &mut ctx.accounts.nft_listing;
        require!(nft_listing.is_listed, ErrorCode::NFTNotListed);

        nft_listing.is_listed = false;
        msg!("NFT has been delisted!");

        emit!(NftDelisted {
            seller: *ctx.accounts.seller.key,
            mint: *ctx.accounts.mint.key,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct MintNFT<'info> {
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    /// CHECK: Not dangerous as we don't read/write
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(mut)]
    pub token_account: UncheckedAccount<'info>,
    pub token_metadata_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ListNFT<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is the mint account
    pub mint: UncheckedAccount<'info>,
    #[account(init, payer = seller, space = 8 + 64)]
    pub nft_listing: Account<'info, NFTListing>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PurchaseNFT<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub seller: UncheckedAccount<'info>,
    /// CHECK: Mint account
    pub mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub nft_listing: Account<'info, NFTListing>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DelistNFT<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, close = seller)]
    pub nft_listing: Account<'info, NFTListing>,
}

#[account]
pub struct NFTListing {
    pub seller: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
    pub is_listed: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("NFT mint failed!")]
    MintFailed,
    #[msg("Metadata account creation failed!")]
    MetadataCreateFailed,
    #[msg("Insufficient funds to purchase NFT")]
    InsufficientFunds,
    #[msg("NFT is not listed for sale")]
    NFTNotListed,
}

#[event]
pub struct NftListed {
    pub seller: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
}

#[event]
pub struct NftSold {
    pub seller: Pubkey,
    pub buyer: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
}

#[event]
pub struct NftDelisted {
    pub seller: Pubkey,
    pub mint: Pubkey,
}