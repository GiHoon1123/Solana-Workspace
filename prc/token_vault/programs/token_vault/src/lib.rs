use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("FWu6vgDTxbxxMYVJ5JYoBy8rgebWXB4TYXfDeR4TT1hg");

// -----------------------------------------------------------------------------
// 프로그램 모듈
// -----------------------------------------------------------------------------
// #[program]: 이 모듈의 함수들을 온체인 entrypoint로 변환
#[program]
pub mod token_vault {
    use super::*;

    // Vault 초기화 (최초 1회)
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.vault_token_account = ctx.accounts.vault_token_account.key();
        
        msg!("Vault 생성 완료!");
        msg!("Owner: {}", vault.owner);
        msg!("Token Account: {}", vault.vault_token_account);
        
        Ok(())
    }

    // 사용자 금고 계정 초기화 (사용자별 최초 1회)
    pub fn initialize_user_vault(ctx: Context<InitializeUserVault>) -> Result<()> {
        let user_vault = &mut ctx.accounts.user_vault_account;
        user_vault.user = ctx.accounts.user.key();
        user_vault.vault = ctx.accounts.vault.key();
        user_vault.deposited_amount = 0;
        
        msg!("사용자 금고 계정 생성 완료!");
        msg!("사용자: {}", user_vault.user);
        
        Ok(())
    }

    // 토큰 입금
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        // CPI: Token Program을 호출해서 토큰 전송
        // 사용자 Token Account → Vault Token Account
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // 사용자 입금 기록 업데이트
        let user_vault = &mut ctx.accounts.user_vault_account;
        user_vault.deposited_amount = user_vault.deposited_amount.checked_add(amount)
            .ok_or(VaultError::Overflow)?;

        msg!("입금 완료!");
        msg!("사용자: {}", user_vault.user);
        msg!("입금액: {}", amount);
        msg!("총 잔액: {}", user_vault.deposited_amount);

        Ok(())
    }

    // 토큰 출금
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let user_vault = &ctx.accounts.user_vault_account;

        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            user_vault.deposited_amount >= amount,
            VaultError::InsufficientBalance
        );

        // Vault PDA로 서명해서 토큰 전송
        // Vault Token Account → 사용자 Token Account
        let seeds = &[b"vault".as_ref(), &[ctx.bumps.vault]];
        let signer_seeds = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        // 사용자 잔액 감소
        let user_vault = &mut ctx.accounts.user_vault_account;
        user_vault.deposited_amount = user_vault.deposited_amount.checked_sub(amount)
            .ok_or(VaultError::Underflow)?;

        msg!("출금 완료!");
        msg!("사용자: {}", user_vault.user);
        msg!("출금액: {}", amount);
        msg!("남은 잔액: {}", user_vault.deposited_amount);

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Context 구조체들
// -----------------------------------------------------------------------------

// Vault 초기화
// #[derive(Accounts)]: Anchor가 계정 파싱 및 검증 코드 자동 생성
#[derive(Accounts)]
pub struct InitializeVault<'info> {
    // #[account(mut)]: 이 계정은 수정 가능 (수수료 지불)
    #[account(mut)]
    pub owner: Signer<'info>,

    // #[account(...)]: 계정 제약 조건 정의
    #[account(
        init,                              // 새 계정 생성
        payer = owner,                     // owner가 계정 생성 비용 지불
        space = 8 + Vault::INIT_SPACE,     // 계정 크기
        seeds = [b"vault"],                // PDA seeds
        bump                               // PDA bump (자동 계산)
    )]
    pub vault: Account<'info, Vault>,

    // Vault의 Token Account (실제 토큰이 보관되는 곳)
    pub vault_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
}

// 사용자 금고 계정 초기화
#[derive(Accounts)]
pub struct InitializeUserVault<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // Vault PDA (읽기 전용)
    #[account(
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,

    // 사용자별 입금 기록 (새로 생성)
    #[account(
        init,
        payer = user,
        space = 8 + UserVaultAccount::INIT_SPACE,
        seeds = [b"user-vault", user.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    pub user_vault_account: Account<'info, UserVaultAccount>,

    pub system_program: Program<'info, System>,
}

// 토큰 입금
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // 사용자의 Token Account (토큰이 빠져나가는 곳)
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    // Vault PDA (읽기 전용)
    #[account(
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,

    // Vault의 Token Account (토큰이 들어가는 곳)
    #[account(
        mut,
        constraint = vault_token_account.key() == vault.vault_token_account
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // 사용자별 입금 기록 (이미 존재해야 함)
    #[account(
        mut,
        seeds = [b"user-vault", user.key().as_ref(), vault.key().as_ref()],
        bump,
        constraint = user_vault_account.user == user.key()
    )]
    pub user_vault_account: Account<'info, UserVaultAccount>,

    pub token_program: Program<'info, Token>,
}

// 토큰 출금
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // 사용자의 Token Account (토큰이 들어가는 곳)
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    // Vault PDA (토큰 전송 권한자)
    #[account(
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,

    // Vault의 Token Account (토큰이 빠져나가는 곳)
    #[account(
        mut,
        constraint = vault_token_account.key() == vault.vault_token_account
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // 사용자별 입금 기록 (잔액 확인 및 차감)
    #[account(
        mut,
        seeds = [b"user-vault", user.key().as_ref(), vault.key().as_ref()],
        bump,
        constraint = user_vault_account.user == user.key()
    )]
    pub user_vault_account: Account<'info, UserVaultAccount>,

    pub token_program: Program<'info, Token>,
}

// -----------------------------------------------------------------------------
// 계정 데이터 구조체
// -----------------------------------------------------------------------------

// Vault (금고 정보)
// #[account]: 이 구조체를 온체인 계정 데이터로 사용 (직렬화/역직렬화 자동)
#[account]
// #[derive(InitSpace)]: 계정 크기 자동 계산 (INIT_SPACE 상수 생성)
#[derive(InitSpace)]
pub struct Vault {
    pub owner: Pubkey,              // 금고 소유자
    pub vault_token_account: Pubkey, // 토큰이 보관되는 계정
}

// UserVaultAccount (사용자별 입금 기록)
#[account]
#[derive(InitSpace)]
pub struct UserVaultAccount {
    pub user: Pubkey,           // 사용자 주소
    pub vault: Pubkey,          // 어느 Vault인지
    pub deposited_amount: u64,  // 입금한 총액
}

// -----------------------------------------------------------------------------
// 에러 정의
// -----------------------------------------------------------------------------
// #[error_code]: 에러 enum을 Anchor 에러로 변환 (자동 에러 코드 생성)
#[error_code]
pub enum VaultError {
    // #[msg(...)]: 에러 발생 시 표시될 메시지
    #[msg("금액은 0보다 커야 합니다")]
    InvalidAmount,

    #[msg("잔액이 부족합니다")]
    InsufficientBalance,

    #[msg("계산 오버플로우")]
    Overflow,

    #[msg("계산 언더플로우")]
    Underflow,
}
