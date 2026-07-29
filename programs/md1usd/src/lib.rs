#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "MD1usd Stablecoin",
    project_url: "https://yourdomain.com",
    contacts: "email:security@yourdomain.com",
    policy: "https://yourdomain.com/security-policy",
    preferred_language: "en",
    acknowledgements: "Audited by Security Team"
}

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("Md1usdSoLXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod md1usd {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        require!(
            ctx.accounts.asset_mint.key() != ctx.accounts.md1usd_mint.key(),
            ErrorCode::InvalidAssetMint
        );

        ctx.accounts.config.asset_mint = ctx.accounts.asset_mint.key();
        ctx.accounts.config.md1usd_mint = ctx.accounts.md1usd_mint.key();

        emit!(Initialized {
            asset_mint: ctx.accounts.asset_mint.key(),
            md1usd_mint: ctx.accounts.md1usd_mint.key(),
        });

        Ok(())
    }

    pub fn mint(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountMustBeGreaterThanZero);

        let balance_before = ctx.accounts.vault_account.amount;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_asset_account.to_account_info(),
                    to: ctx.accounts.vault_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        ctx.accounts.vault_account.reload()?;
        let balance_after = ctx.accounts.vault_account.amount;

        let actually_received = balance_after
            .checked_sub(balance_before)
            .ok_or(ErrorCode::MathOverflow)?;
        require!(actually_received > 0, ErrorCode::NoFundsReceived);

        let bump = ctx.bumps.mint_authority;
        let seeds = &[b"mint_authority".as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.md1usd_mint.to_account_info(),
                    to: ctx.accounts.user_md1usd_account.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                signer,
            ),
            actually_received,
        )?;

        emit!(Minted {
            user: ctx.accounts.user.key(),
            amount: actually_received,
        });

        Ok(())
    }

    pub fn redeem(ctx: Context<RedeemTokens>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::AmountMustBeGreaterThanZero);
        require!(
            ctx.accounts.user_md1usd_account.amount >= amount,
            ErrorCode::InsufficientBalance
        );

        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.md1usd_mint.to_account_info(),
                    from: ctx.accounts.user_md1usd_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        let bump = ctx.bumps.vault_authority;
        let seeds = &[b"vault_authority".as_ref(), &[bump]];
        let signer = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_account.to_account_info(),
                    to: ctx.accounts.user_asset_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer,
            ),
            amount,
        )?;

        emit!(Redeemed {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
}

#[account]
pub struct Config {
    pub asset_mint: Pubkey,
    pub md1usd_mint: Pubkey,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub asset_mint: Account<'info, Mint>,

    /// CHECK: PDA بدون بيانات
    #[account(seeds = [b"vault_authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        token::mint = asset_mint,
        token::authority = vault_authority,
        seeds = [b"vault", asset_mint.key().as_ref()],
        bump
    )]
    pub vault_account: Account<'info, TokenAccount>,

    /// CHECK: PDA بدون بيانات
    #[account(seeds = [b"mint_authority"], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = asset_mint.decimals,
        mint::authority = mint_authority,
        seeds = [b"md1usd_mint"],
        bump
    )]
    pub md1usd_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_asset_account.owner == user.key(),
        constraint = user_asset_account.mint == asset_mint.key()
    )]
    pub user_asset_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_md1usd_account.owner == user.key(),
        constraint = user_md1usd_account.mint == md1usd_mint.key()
    )]
    pub user_md1usd_account: Account<'info, TokenAccount>,

    pub asset_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"vault", asset_mint.key().as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = asset_mint
    )]
    pub vault_account: Account<'info, TokenAccount>,

    /// CHECK: PDA authority only
    #[account(seeds = [b"vault_authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"md1usd_mint"],
        bump
    )]
    pub md1usd_mint: Account<'info, Mint>,

    /// CHECK: PDA authority only
    #[account(seeds = [b"mint_authority"], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [b"config"],
        bump,
        constraint = config.asset_mint == asset_mint.key(),
        constraint = config.md1usd_mint == md1usd_mint.key()
    )]
    pub config: Account<'info, Config>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RedeemTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_asset_account.owner == user.key(),
        constraint = user_asset_account.mint == asset_mint.key()
    )]
    pub user_asset_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_md1usd_account.owner == user.key(),
        constraint = user_md1usd_account.mint == md1usd_mint.key()
    )]
    pub user_md1usd_account: Account<'info, TokenAccount>,

    pub asset_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"vault", asset_mint.key().as_ref()],
        bump,
        token::authority = vault_authority,
        token::mint = asset_mint
    )]
    pub vault_account: Account<'info, TokenAccount>,

    /// CHECK: PDA authority only
    #[account(seeds = [b"vault_authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"md1usd_mint"],
        bump
    )]
    pub md1usd_mint: Account<'info, Mint>,

    #[account(
        seeds = [b"config"],
        bump,
        constraint = config.asset_mint == asset_mint.key(),
        constraint = config.md1usd_mint == md1usd_mint.key()
    )]
    pub config: Account<'info, Config>,

    pub token_program: Program<'info, Token>,
}

#[event]
pub struct Minted {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Redeemed {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Initialized {
    pub asset_mint: Pubkey,
    pub md1usd_mint: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    AmountMustBeGreaterThanZero,
    #[msg("No funds received")]
    NoFundsReceived,
    #[msg("Insufficient MD1usd balance")]
    InsufficientBalance,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Asset mint cannot be the same as MD1usd mint")]
    InvalidAssetMint,
}
