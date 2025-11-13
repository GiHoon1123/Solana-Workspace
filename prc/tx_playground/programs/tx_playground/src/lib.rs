use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};

/// Maximum number of aggregators allowed.
/// 허용할 어그리게이터의 최대 개수.
pub const MAX_AGGREGATORS: usize = 2;

/// Fixed byte-length used to store each aggregator name.
/// 각 어그리게이터 이름을 저장할 때 사용할 고정 바이트 길이.
pub const AGGREGATOR_NAME_LEN: usize = 16;

/// Human-readable prefix when creating TxLog PDA seeds.
/// TxLog PDA 씨드를 생성할 때 사용할 사람이 읽기 좋은 접두어.
pub const TX_LOG_SEED_PREFIX: &[u8] = b"tx_log";

declare_id!("BboWCo5mn96epXaSmq3prjAhx1tKkFZdjjyGgMGVcckW");

#[program]
pub mod tx_playground {
    use super::*;

    // 프로그램 설정을 초기화 한다. (owner, whitelisted, aggregator)
    // initialize program configuration (owner, whitelisted, aggregator)
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        aggregators: Vec<AggregatorName>,
    ) -> Result<()> {
        msg!("initialize_config called: setting program configuration");

        // Throw error if too many aggregators are provided
        require!(
            aggregators.len() <= MAX_AGGREGATORS,
            TxPlaygroundError::TooManyAggregators
        );

        // Throw error if no aggregators are provided
        require!(
            !aggregators.is_empty(),
            TxPlaygroundError::NoAggregatorProvided
        );

        // Get authority from context
        let authority = &ctx.accounts.authority;
        let config = &mut ctx.accounts.config;

        // Set owner, bump, aggregator_count
        config.owner = authority.key();
        config.bump = ctx.bumps.config;
        config.aggregator_count = aggregators.len() as u8;

        // Set AggregatorName to default
        for slot in config.aggregators.iter_mut() {
            *slot = AggregatorName::default();
        }

        // Set aggregator names
        for (idx, name) in aggregators.into_iter().enumerate() {
            config.aggregators[idx] = name;
        }

        Ok(())
    }

    // 사용자 상태를 초기화 한다.
    // Initialize user state 
    pub fn initialize_user_state(ctx: Context<InitializeUserState>) -> Result<()> {
        msg!("initialize_user_state called: creating per-user state");

        
        // Get user state from context
        let user_state = &mut ctx.accounts.user_state;
        user_state.last_tx_id = 0;
        user_state.bump = ctx.bumps.user_state;

        Ok(())
    }

   // Transfer tokens from user to destination
    pub fn transfer(
        ctx: Context<Transfer>,
        amount: u64,
        log_seed: [u8; 8],
    ) -> Result<()> {
        // Throw error if amount is not greater than 0 
        msg!("transfer called: moving SPL tokens");
        require!(amount > 0, TxPlaygroundError::InvalidAmount);

        // Convert log_seed to log_id
        let log_id = u64::from_le_bytes(log_seed);

        // Check if log_id is valid 
        let user_state = &mut ctx.accounts.user_state;
        user_state.consume_log_id(log_id)?;

        msg!("tx_log account provided: {}", ctx.accounts.tx_log.key());
        msg!("log_id value: {}", log_id);

        
        // Each of Solana Fuctions exists as a separate program, so we need to use CpiContext to call them.
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: ctx.accounts.destination_token.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        // Get clock from Solana runtime timestamp
        let clock = Clock::get()?;

        // Record transaction metadata into on-chain Txlog account
        let tx_log = &mut ctx.accounts.tx_log;

        tx_log.user = ctx.accounts.authority.key();
        tx_log.mode = TxMode::Transfer;
        tx_log.aggregator = AggregatorName::default();
        tx_log.amount_in = amount;
        tx_log.amount_out = amount;
        tx_log.timestamp = clock.unix_timestamp;
        tx_log.bump = ctx.bumps.tx_log;

        Ok(())
    }

    // Swap tokens manually
    pub fn manual_swap(
        ctx: Context<ManualSwap>,
        amount_in: u64,
        expected_amount_out: u64, // Expected amount out after swap 
        log_seed: [u8; 8],
    ) -> Result<()> {
        msg!("manual_swap called: executing direct swap");
        require!(amount_in > 0, TxPlaygroundError::InvalidAmount);
        require!(expected_amount_out > 0, TxPlaygroundError::InvalidAmount);

        let log_id = u64::from_le_bytes(log_seed);
        let user_state = &mut ctx.accounts.user_state;
        user_state.consume_log_id(log_id)?;

        msg!("tx_log account provided: {}", ctx.accounts.tx_log.key());
        msg!("log_id value: {}", log_id);

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: ctx.accounts.destination_token.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount_in,
        )?;

        let clock = Clock::get()?;
        let tx_log = &mut ctx.accounts.tx_log;

        tx_log.user = ctx.accounts.authority.key();
        tx_log.mode = TxMode::ManualSwap;
        tx_log.aggregator = AggregatorName::default();
        tx_log.amount_in = amount_in;
        tx_log.amount_out = expected_amount_out;
        tx_log.timestamp = clock.unix_timestamp;
        tx_log.bump = ctx.bumps.tx_log;

        Ok(())
    }

    pub fn aggregator_swap(
        ctx: Context<AggregatorSwap>,
        aggregator: AggregatorName, // Aggregator name to use for swap
        amount_in: u64,
        min_amount_out: u64,
        log_seed: [u8; 8],
    ) -> Result<()> {
        msg!("aggregator_swap called: logging aggregator execution");
        // Throw error if amount is not greater than 0 
        require!(amount_in > 0, TxPlaygroundError::InvalidAmount);

        // Throw error if amount is not greater than 0
        require!(min_amount_out > 0, TxPlaygroundError::InvalidAmount);

        // Get config from context
        let config = &ctx.accounts.config;

        // Get active aggregator from config
        let active = &config.aggregators[..config.aggregator_count as usize];
        
        // Throw error if aggregator is not whitelisted inf config 
        require!(
            active.contains(&aggregator),
            TxPlaygroundError::AggregatorNotWhitelisted
        );

        let log_id = u64::from_le_bytes(log_seed);
        let user_state = &mut ctx.accounts.user_state;
        user_state.consume_log_id(log_id)?;

        msg!("tx_log account provided: {}", ctx.accounts.tx_log.key());
        msg!("log_id value: {}", log_id);

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SplTransfer {
                    from: ctx.accounts.user_source.to_account_info(),
                    to: ctx.accounts.destination_token.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount_in,
        )?;

        let clock = Clock::get()?;
        let tx_log = &mut ctx.accounts.tx_log;

        tx_log.user = ctx.accounts.authority.key();
        tx_log.mode = TxMode::AggregatorSwap;
        tx_log.aggregator = aggregator;
        tx_log.amount_in = amount_in;
        tx_log.amount_out = min_amount_out;
        tx_log.timestamp = clock.unix_timestamp;
        tx_log.bump = ctx.bumps.tx_log;

        Ok(())
    }
}

#[derive(Accounts)] // Validate the account is valid and owned by the program
pub struct InitializeConfig<'info> {
    #[account(mut)] // Mutably access the account 
    pub authority: Signer<'info>,
    #[account(
        init, // Create a new account for the config
        payer = authority, // Payer is the authority
        space = 8 + Config::INIT_SPACE, //  Set the space for the config account
        seeds = [Config::SEED_PREFIX], // Use the seed prefix for the config account
        bump
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>, // System program to create the config account 
}

#[derive(Accounts)]
pub struct InitializeUserState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = authority,
        space = 8 + UserState::INIT_SPACE,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump
    )]
    pub user_state: Account<'info, UserState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: u64, log_seed: [u8; 8])] // Used to provide extra parmeters needed for account validation.
pub struct Transfer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,
    #[account(mut)]
    pub user_source: Account<'info, TokenAccount>, // The user's SPL token account that holds the tokens to be transferred.
    #[account(mut)]
    pub destination_token: Account<'info, TokenAccount>, // The destination SPL token account to receive the tokens.
    #[account(
        init,
        payer = authority,
        space = 8 + TxLog::INIT_SPACE,
        seeds = [
            TX_LOG_SEED_PREFIX,
            authority.key().as_ref(),
            &log_seed
        ],
        bump
    )]
    pub tx_log: Account<'info, TxLog>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount_in: u64, expected_amount_out: u64, log_seed: [u8; 8])]
pub struct ManualSwap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,
    #[account(mut)]
    pub user_source: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_token: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        space = 8 + TxLog::INIT_SPACE,
        seeds = [
            TX_LOG_SEED_PREFIX,
            authority.key().as_ref(),
            &log_seed
        ],
        bump
    )]
    pub tx_log: Account<'info, TxLog>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(aggregator: AggregatorName, amount_in: u64, min_amount_out: u64, log_seed: [u8; 8])]
pub struct AggregatorSwap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds = [Config::SEED_PREFIX], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [UserState::SEED_PREFIX, authority.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,
    #[account(mut)]
    pub user_source: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_token: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        space = 8 + TxLog::INIT_SPACE,
        seeds = [
            TX_LOG_SEED_PREFIX,
            authority.key().as_ref(),
            &log_seed
        ],
        bump
    )]
    pub tx_log: Account<'info, TxLog>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub owner: Pubkey,
    pub aggregators: [AggregatorName; MAX_AGGREGATORS],
    pub aggregator_count: u8,
    pub bump: u8,
}

impl Config {
    pub const SEED_PREFIX: &'static [u8] = b"config";
}

#[account]
#[derive(InitSpace)]
pub struct UserState {
    pub last_tx_id: u64,
    pub bump: u8,
}

impl UserState {
    pub const SEED_PREFIX: &'static [u8] = b"user_state";

    pub fn consume_log_id(&mut self, log_id: u64) -> Result<()> {
        let expected = self
            .last_tx_id
            .checked_add(1)
            .ok_or_else(|| error!(TxPlaygroundError::CounterOverflow))?;
        require!(log_id == expected, TxPlaygroundError::InvalidLogId);
        self.last_tx_id = log_id;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct TxLog {
    pub user: Pubkey,
    pub mode: TxMode,
    pub aggregator: AggregatorName,
    pub amount_in: u64,
    pub amount_out: u64,
    pub timestamp: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregatorName {
    pub data: [u8; AGGREGATOR_NAME_LEN],
}

impl AggregatorName {
    pub fn from_str(name: &str) -> Result<Self> {
        let bytes = name.as_bytes();
        require!(
            bytes.len() <= AGGREGATOR_NAME_LEN,
            TxPlaygroundError::AggregatorNameTooLong
        );

        let mut fixed = [0u8; AGGREGATOR_NAME_LEN];
        fixed[..bytes.len()].copy_from_slice(bytes);
        Ok(Self { data: fixed })
    }
}

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

#[error_code]
pub enum TxPlaygroundError {
    #[msg("Too many aggregators provided")]
    TooManyAggregators,
    #[msg("At least one aggregator must be provided")]
    NoAggregatorProvided,
    #[msg("Aggregator name exceeds maximum length")]
    AggregatorNameTooLong,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Counter overflow while generating log ID")]
    CounterOverflow,
    #[msg("Aggregator is not whitelisted in config")]
    AggregatorNotWhitelisted,
    #[msg("Provided log id does not match expected sequence")]
    InvalidLogId,
}

impl anchor_lang::Space for AggregatorName {
    const INIT_SPACE: usize = AGGREGATOR_NAME_LEN;
}

impl anchor_lang::Space for TxMode {
    const INIT_SPACE: usize = 1;
}
