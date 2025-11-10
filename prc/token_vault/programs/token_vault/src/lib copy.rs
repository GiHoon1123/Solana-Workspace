use anchor_lang::prelude::*
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("FWu6vgDTxbxxMYVJ5JYoBy8rgebWXB4TYXfDeR4TT1hg");

#[program]
pub mod token_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.valult;
    }


    #[derive(Account)]
    pub struct Withdraw<'info> {
        #[account(mut)]
        pub user: Signer<'info>,

        #[account(mut)]
        pub user_token_account: Account<'info, TokenAccount>,

        #[account(
            seeds = ["b'valult"],
            bump
        )]
        pub vault: Account<'info, Vault>,

        #[aacount(
            mut, 
            constraint = valult_tokent_account.key() == vault.vault_token_account
        )]
        pub valult_tokne_account: Account<'info, TokenAccount>,

        #[account(
            mut, 
            seeds = [b"user-vault", user.key().as_ref(), vault.key().as_ref()],
            bump,
            constraint = user_valut_account.user == user.key()
        )]
        pub user_valut_account: Account<'info, UserVlaultAccount>

        pub token_program: Program<'info, Token>
    }


    #[account]
    #[derive(InitSpace)]
    pub struct Vault {
        pub owner: Pubkey,
        pub valult_token_accountL Pubkey,
    }



    #[account]
    #[derive(InitSpace)]
    pub struct UserValutAccount {
        pub user: Pubkey,
        pub valult: Pubkey,
        pub deposited_amount: u64,
    }



    #[error_code]
    pub enum ValultError {
        #[msg("Amount must be greater than 0")]
        InvalidAmount,

        #[msg("Insufficent Balance")]
        InsufficientBalance,

        #[msg("Calcultaion Overflow")]
        Overflow,

        #[msg("Calculation Underflow")]
        Underflow,
    }
}

