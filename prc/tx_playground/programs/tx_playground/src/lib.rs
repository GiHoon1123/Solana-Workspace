use anchor_lang::prelude::*;

/// Maximum number of aggregators allowed.
/// 허용할 어그리게이터의 최대 개수.
pub const MAX_AGGREGATORS: usize = 2;

/// Fixed byte-length used to store each aggregator name.
/// 각 어그리게이터 이름을 저장할 때 사용할 고정 바이트 길이.
pub const AGGREGATOR_NAME_LEN: usize = 16;

declare_id!("BboWCo5mn96epXaSmq3prjAhx1tKkFZdjjyGgMGVcckW");

#[program]
pub mod tx_playground {
    use super::*;

    /// Admin initializes config and registers allowed aggregators.
    /// 관리자가 컨피그 계정을 초기화하고 허용된 어그리게이터를 등록한다.
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        aggregators: Vec<AggregatorName>,
    ) -> Result<()> {
        msg!("initialize_config called. TODO: implement logic");

        // Reject when provided aggregators exceed the allowed maximum.
        // 전달된 어그리게이터 수가 허용 범위를 넘으면 에러 처리.
        require!(
            aggregators.len() <= MAX_AGGREGATORS,
            TxPlaygroundError::TooManyAggregators
        );

        let config = &mut ctx.accounts.config;
        config.owner = ctx.accounts.authority.key();
        config.bump = ctx.bumps.config;
        config.aggregator_count = aggregators.len() as u8;

        // Copy each aggregator name into the fixed-length array.
        // 각 어그리게이터 이름을 고정 길이 배열에 복사.
        for (idx, name) in aggregators.into_iter().enumerate() {
            config.aggregators[idx] = name;
        }

        Ok(())
    }

    /// Basic transfer; later we will log the transaction details.
    /// 기본 토큰/SOL 전송; 이후 트랜잭션 로그를 남길 예정이다.
    pub fn transfer(ctx: Context<Transfer>, _amount: u64) -> Result<()> {
        msg!("transfer called. TODO: implement token/native transfer logic");
        Ok(())
    }

    /// Manual swap supplied with off-chain price calculations.
    /// 오프체인 가격 계산을 받아 수동 스왑을 수행한다.
    pub fn manual_swap(
        ctx: Context<ManualSwap>,
        _amount_in: u64,
        _min_amount_out: u64,
    ) -> Result<()> {
        msg!("manual_swap called. TODO: implement manual swap execution");
        Ok(())
    }

    /// Swap executed through a whitelisted aggregator.
    /// 허용된 어그리게이터를 통해 스왑을 수행한다.
    pub fn aggregator_swap(
        ctx: Context<AggregatorSwap>,
        _aggregator: AggregatorName,
        _amount_in: u64,
        _min_amount_out: u64,
    ) -> Result<()> {
        msg!("aggregator_swap called. TODO: integrate with external aggregator");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Authority that creates the config account.
    /// 컨피그 계정을 생성하는 권한자.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Config PDA stored with program-wide settings.
    /// 프로그램 전역 설정을 저장하는 Config PDA.
    #[account(
        init,
        payer = authority,
        space = 8 + Config::INIT_SPACE,
        seeds = [Config::SEED_PREFIX],
        bump
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    /// User or admin executing the transfer.
    /// 전송을 수행하는 사용자 또는 관리자.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Config PDA loaded for validation.
    /// 검증을 위해 불러오는 Config PDA.
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,

    /// Per-user state storing counters for logging.
    /// 로그 작성을 위한 사용자별 상태(카운터 등)를 저장.
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + UserState::INIT_SPACE,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump
    )]
    pub user_state: Account<'info, UserState>,

    /// Transaction log account to be replaced with a PDA later.
    /// 향후 PDA로 전환할 트랜잭션 로그 계정.
    #[account(mut)]
    pub tx_log: Account<'info, TxLog>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManualSwap<'info> {
    /// User initiating the manual swap.
    /// 수동 스왑을 실행하는 사용자.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Config PDA ensuring the program configuration.
    /// 프로그램 설정을 보장하는 Config PDA.
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,

    /// User state PDA tracking swap counters.
    /// 스왑 카운터를 추적하는 사용자 상태 PDA.
    #[account(
        mut,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,

    /// Transaction log account for recording swap events.
    /// 스왑 이벤트를 기록할 트랜잭션 로그 계정.
    #[account(mut)]
    pub tx_log: Account<'info, TxLog>,
}

#[derive(Accounts)]
pub struct AggregatorSwap<'info> {
    /// User calling the aggregator swap.
    /// 어그리게이터 스왑을 호출하는 사용자.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Config PDA used to verify allowed aggregators.
    /// 허용된 어그리게이터를 검증하는 Config PDA.
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,

    /// User state PDA storing aggregator swap counters.
    /// 어그리게이터 스왑 카운터를 저장하는 사용자 상태 PDA.
    #[account(
        mut,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,

    /// Transaction log account recording aggregator usage.
    /// 어그리게이터 사용 내역을 기록하는 트랜잭션 로그 계정.
    #[account(mut)]
    pub tx_log: Account<'info, TxLog>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Owner authorized to update program configuration.
    /// 프로그램 설정을 갱신할 수 있는 오너.
    pub owner: Pubkey,

    /// Fixed-size array of allowed aggregators.
    /// 허용된 어그리게이터를 담는 고정 길이 배열.
    pub aggregators: [AggregatorName; MAX_AGGREGATORS],

    /// Number of aggregators currently registered.
    /// 현재 등록된 어그리게이터 수.
    pub aggregator_count: u8,

    /// PDA bump value for config derivation.
    /// Config PDA 도출에 사용되는 bump 값.
    pub bump: u8,
}

impl Config {
    /// PDA seed prefix for the global config account.
    /// 전역 Config 계정을 위한 PDA 씨드 접두어.
    pub const SEED_PREFIX: &'static [u8] = b"config";
}

#[account]
#[derive(InitSpace)]
pub struct UserState {
    /// Sequential identifier used to derive log PDAs.
    /// 로그 PDA를 만들 때 쓰는 순차적 식별자.
    pub last_tx_id: u64,

    /// PDA bump value for the user state account.
    /// 사용자 상태 PDA에 대한 bump 값.
    pub bump: u8,
}

impl UserState {
    /// PDA seed prefix for individual user state.
    /// 각 사용자 상태를 위한 PDA 씨드 접두어.
    pub const SEED_PREFIX: &'static [u8] = b"user_state";
}

#[account]
#[derive(InitSpace)]
pub struct TxLog {
    /// User who initiated the transaction.
    /// 트랜잭션을 실행한 사용자.
    pub user: Pubkey,

    /// Type of transaction executed (transfer/swap/etc).
    /// 실행된 트랜잭션 종류 (전송/스왑 등).
    pub mode: TxMode,

    /// Aggregator involved; default zeroed for pure transfer.
    /// 관여된 어그리게이터; 단순 전송일 경우 기본값(0) 유지.
    pub aggregator: AggregatorName,

    /// Input amount supplied by the user.
    /// 사용자가 투입한 금액.
    pub amount_in: u64,

    /// Amount received after the operation.
    /// 작업 후 수령한 금액.
    pub amount_out: u64,

    /// Unix timestamp recorded for historical analysis.
    /// 히스토리 분석을 위한 유닉스 타임스탬프.
    pub timestamp: i64,
}

/// Fixed-size wrapper used to store aggregator names.
/// 어그리게이터 이름을 저장하기 위한 고정 길이 래퍼.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregatorName(pub [u8; AGGREGATOR_NAME_LEN]);

/// Enumeration describing each transaction mode.
/// 각 트랜잭션 모드를 설명하는 열거형.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TxMode {
    Transfer,
    ManualSwap,
    AggregatorSwap,
}

impl Default for TxMode {
    fn default() -> Self {
        TxMode::Transfer
    }
}

/// Custom program errors raised by tx_playground.
/// tx_playground에서 발생시키는 사용자 정의 에러.
#[error_code]
pub enum TxPlaygroundError {
    #[msg("Too many aggregators provided")]
    TooManyAggregators,
}