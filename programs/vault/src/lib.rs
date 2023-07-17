use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

declare_id!("DpLHaRUPhCru3F8f3Aa1V8xHAxKmb9cdEqFD3E9BHRXv");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>, deposit_amount: u64) -> Result<()> {
        // ensure deposit amount is greater than 0
        if deposit_amount <= 0 {
            return err!(ErrorCode::InvalidDepositAmount);
        }

        msg!("depositing {} to vault", deposit_amount);
        // Transfer token from the vault owner to the vault token account
        let context = ctx.accounts.token_program_context(Transfer {
            from: ctx.accounts.owner_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        });
        transfer(context, deposit_amount)?;

        let vault_bump = *ctx.bumps.get("vault").unwrap();
        let vault_authority_bump = *ctx.bumps.get("vault_authority").unwrap();
        let vault_token_account_bump = *ctx.bumps.get("vault_token_account").unwrap();
        let bumps = Bumps {
            vault: vault_bump,
            vault_authority: vault_authority_bump,
            vault_token_account: vault_token_account_bump,
        };
        ctx.accounts.vault.set_inner(Vault {
            deposited_amount: deposit_amount,
            withdrawn_amount: 0,
            initialized: true,
            owner: ctx.accounts.owner.key(),
            mint: ctx.accounts.mint.key(),
            bumps,
        });
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, deposit_amount: u64) -> Result<()> {
        // ensure deposit amount is greater than 0
        if deposit_amount <= 0 {
            return err!(ErrorCode::InvalidDepositAmount);
        }

        msg!("depositing {} to vault", deposit_amount);
        // Transfer token from the vault owner to the vault token account
        let context = ctx.accounts.token_program_context(Transfer {
            from: ctx.accounts.owner_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        });
        transfer(context, deposit_amount)?;

        let vault_data = &mut ctx.accounts.vault;
        vault_data.deposited_amount += deposit_amount;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, withdraw_amount: u64) -> Result<()> {
        let vault_token_balance = &ctx.accounts.vault_token_account.amount;
        if vault_token_balance < &withdraw_amount || withdraw_amount <= 0 {
            return err!(ErrorCode::InvalidWithdrawAmount);
        }
        msg!("Withdrawing {} to owner account", withdraw_amount);
        let release_to_owner = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.owner_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        transfer(
            ctx.accounts
                .token_program_context(release_to_owner)
                .with_signer(&[&[
                    b"authority",
                    ctx.accounts.vault.key().as_ref(),
                    &[ctx.accounts.vault.bumps.vault_authority],
                ]]),
            withdraw_amount,
        )?;

        let vault_data = &mut ctx.accounts.vault;
        vault_data.withdrawn_amount += withdraw_amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    // external accounts
    #[account(mut)]
    owner: Signer<'info>,
    #[account(constraint = mint.is_initialized == true)]
    mint: Account<'info, Mint>,
    #[account(mut, token::mint=mint, token::authority=owner)]
    owner_token_account: Account<'info, TokenAccount>,

    // PDAs
    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [b"vault".as_ref(), owner.key().as_ref(), mint.key().as_ref()], bump
    )]
    vault: Account<'info, Vault>,
    #[account(
        seeds = [b"authority".as_ref(), vault.key().as_ref()], bump
    )]
    vault_authority: SystemAccount<'info>,
    #[account(
        init,
        payer = owner,
        token::mint=mint,
        token::authority=vault_authority,
        seeds = [b"tokens".as_ref(), vault.key().as_ref()], bump
    )]
    vault_token_account: Account<'info, TokenAccount>,
    // Programs
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeVault<'info> {
    pub fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct Bumps {
    pub vault: u8,
    pub vault_authority: u8,
    pub vault_token_account: u8,
}

#[account]
#[derive(Debug)]
pub struct Vault {
    pub deposited_amount: u64,
    pub withdrawn_amount: u64,
    pub initialized: bool,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub bumps: Bumps,
}

impl Vault {
    pub const LEN: usize = {
        let discriminator = 8;
        let amounts = 2 * 8;
        let initialized = 1;
        let pubkeys = 2 * 32;
        let vault_bumps = 3 * 1;
        discriminator + amounts + initialized + pubkeys + vault_bumps
    };
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    // External accounts
    #[account(address = vault.owner)]
    owner: Signer<'info>,
    #[account(mut, token::mint=vault.mint, token::authority=owner)]
    owner_token_account: Account<'info, TokenAccount>,
    #[account(constraint = mint.is_initialized == true)]
    mint: Account<'info, Mint>,

    // PDAs
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        bump = vault.bumps.vault,
        constraint = vault.initialized == true,
    )]
    vault: Account<'info, Vault>,
    #[account(
        seeds = [b"authority".as_ref(), vault.key().as_ref()],
        bump = vault.bumps.vault_authority
    )]
    vault_authority: SystemAccount<'info>,
    #[account(
        mut,
        token::mint=vault.mint,
        token::authority=vault_authority,
        seeds = [b"tokens".as_ref(), vault.key().as_ref()],
        bump = vault.bumps.vault_token_account
    )]
    vault_token_account: Account<'info, TokenAccount>,

    // Programs section
    token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    // External accounts
    #[account(address = vault.owner)]
    owner: Signer<'info>,
    #[account(mut, token::mint=vault.mint, token::authority=owner)]
    owner_token_account: Account<'info, TokenAccount>,
    #[account(constraint = mint.is_initialized == true)]
    mint: Account<'info, Mint>,

    // PDAs
    #[account(
        mut,
        seeds = [b"vault".as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        bump = vault.bumps.vault,
        constraint = vault.initialized == true,
    )]
    vault: Account<'info, Vault>,
    #[account(
        seeds = [b"authority".as_ref(), vault.key().as_ref()],
        bump = vault.bumps.vault_authority
    )]
    vault_authority: SystemAccount<'info>,
    #[account(
        mut,
        token::mint=vault.mint,
        token::authority=vault_authority,
        seeds = [b"tokens".as_ref(), vault.key().as_ref()],
        bump = vault.bumps.vault_token_account
    )]
    vault_token_account: Account<'info, TokenAccount>,

    // Programs section
    token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Deposit amount must be greater than 0")]
    InvalidDepositAmount,

    #[msg("Withdraw amount must be an amount available in the vault")]
    InvalidWithdrawAmount,
}
