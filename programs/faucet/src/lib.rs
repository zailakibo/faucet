use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::mem::size_of;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum FaucetErrors {
    #[msg("Wait for a while")]
    WaitFor,
}


#[program]
pub mod faucet {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        mint: Pubkey,
        amount: u64,
        timeout: i64,
    ) -> Result<()> {
        ctx.accounts.faucet.amount = amount;
        ctx.accounts.faucet.mint = mint;
        ctx.accounts.faucet.owner = ctx.accounts.payer.key.to_owned();
        ctx.accounts.faucet.timeout = timeout;

        // take the ownership of this TokenAccount
        let cpi_accounts = anchor_spl::token::SetAuthority {
            account_or_mint: ctx.accounts.escrow_wallet.to_account_info(),
            current_authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        let (vault_authority, _bump) = Pubkey::find_program_address(
            &[b"wallet", ctx.accounts.mint.to_account_info().key.as_ref()],
            ctx.program_id,
        );
        anchor_spl::token::set_authority(
            cpi_context,
            anchor_spl::token::spl_token::instruction::AuthorityType::AccountOwner,
            Some(vault_authority),
        )?;
        Ok(())
    }

    pub fn first_airdrop(ctx: Context<FirstAirdrop>) -> Result<()> {
        inner_withdraw(
            ctx.program_id,
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.escrow_wallet.to_account_info(),
            &ctx.accounts.to.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            ctx.accounts.faucet.amount,
        )?;
        let clock = Clock::get()?;
        ctx.accounts.last_drop.to = ctx.accounts.payer.key();
        ctx.accounts.last_drop.timestamp = clock.unix_timestamp;
        Ok(())
    }

    pub fn airdrop(ctx: Context<Airdrop>) -> Result<()> {
        let clock = Clock::get()?;

        // this way is better for future changes on timeout
        let timeout = ctx.accounts.last_drop.timestamp + ctx.accounts.faucet.timeout;
        require!(timeout <= clock.unix_timestamp, FaucetErrors::WaitFor);
        inner_withdraw(
            ctx.program_id,
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.escrow_wallet.to_account_info(),
            &ctx.accounts.to.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            ctx.accounts.faucet.amount,
        )?;
        
        ctx.accounts.last_drop.to = ctx.accounts.payer.key();
        ctx.accounts.last_drop.timestamp = clock.unix_timestamp;
        Ok(())
    }
}

fn inner_withdraw<'a>(
    program_id: &Pubkey,
    mint: &AccountInfo<'a>,
    escrow_wallet: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> Result<()> {
    let (_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[b"wallet", mint.key.as_ref()], program_id);
    let authority_seeds = &[b"wallet", mint.key.as_ref(), &[vault_authority_bump]];
    let signer = &[&authority_seeds[..]];
    let cpi_accounts = anchor_spl::token::Transfer {
        from: escrow_wallet.to_account_info(),
        to: to.to_account_info(),
        authority: escrow_wallet.to_account_info(),
    };
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [
            b"faucet".as_ref(),
            mint.key().as_ref(),
        ],
        bump,
        space = size_of::<Faucet>() + 8
    )]
    faucet: Account<'info, Faucet>,

    #[account(
        init,
        payer = payer,
        seeds = [
            b"wallet".as_ref(),
            mint.key().as_ref(),
        ],
        bump,
        token::mint = mint,
        token::authority = payer,
    )]
    escrow_wallet: Account<'info, TokenAccount>,

    mint: Account<'info, Mint>,

    #[account(mut)]
    payer: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FirstAirdrop<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [
            b"last_drop".as_ref(),
            payer.key().as_ref(),
        ],
        bump,
        space = size_of::<LastDrop>() + 8
    )]
    last_drop: Account<'info, LastDrop>,

    #[account(mut)]
    escrow_wallet: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = to.owner == payer.key()
    )]
    to: Account<'info, TokenAccount>,

    faucet: Account<'info, Faucet>,

    mint: Account<'info, Mint>,

    #[account(mut)]
    payer: Signer<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Airdrop<'info> {
    #[account(mut)]
    last_drop: Account<'info, LastDrop>,

    #[account(mut)]
    escrow_wallet: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = to.owner == payer.key()
    )]
    to: Account<'info, TokenAccount>,

    faucet: Account<'info, Faucet>,

    mint: Account<'info, Mint>,

    #[account(mut)]
    payer: Signer<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[account]
#[derive(Debug, Default)]
pub struct LastDrop {
    to: Pubkey,
    timestamp: i64,
}

#[account]
#[derive(Debug, Default)]
pub struct Faucet {
    owner: Pubkey,
    mint: Pubkey,
    amount: u64,
    timeout: i64,
}
