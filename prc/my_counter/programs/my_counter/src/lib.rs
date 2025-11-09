use anchor_lang::prelude::*;

// -----------------------------------------------------------------------------
// 1. Program ID 선언
// -----------------------------------------------------------------------------
// Solana의 모든 프로그램은 "Program ID"라는 고유 주소로 식별된다.
// Anchor는 빌드 시 Anchor.toml에 명시된 프로그램 ID로 이 값을 자동 교체한다.
// 지금은 placeholder로 작성해 두면 된다.
declare_id!("4QxPT9giJh5BxERUm59LTEpENbJh6mk1xnonBCqVMzJo");

// -----------------------------------------------------------------------------
// 2. #[program] 매크로
// -----------------------------------------------------------------------------
// 이 매크로 아래에 정의된 함수들이 실제 온체인에서 호출 가능한 entrypoint가 된다.
// 즉, 클라이언트(typescript)에서 .methods.함수명() 형태로 호출할 수 있다.
#[program]
pub mod my_counter {
    use super::*;

    // -------------------------------------------------------------------------
    // (1) initialize
    // -------------------------------------------------------------------------
    // 새로운 counter 계정을 생성하고, 내부 count 값을 0으로 초기화한다.
    // Context<Initialize>는 아래에 정의된 Initialize 구조체를 기반으로
    // 이 함수에 필요한 계정 목록을 자동 검증한다.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // (2) increment
    // -------------------------------------------------------------------------
    // 이미 존재하는 counter 계정의 count 값을 1 증가시킨다.
    // Context<Increment>는 counter 계정 하나만 요구한다.
    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// 3. Context 구조체들
// -----------------------------------------------------------------------------
// Context<T>는 함수가 실행될 때 필요한 모든 계정(AccountInfo)을 그룹화한 것이다.
// Anchor는 이 구조체에 정의된 속성들을 기반으로 계정의 유효성 검증을 수행한다.
// (예: init, mut, signer, owner 검증 등)

#[derive(Accounts)]
pub struct Initialize<'info> {
    // 새 counter 계정을 생성한다.
    // - init: 이 계정을 새로 생성한다.
    // - payer = user: rent(계정 생성 수수료)를 user가 지불한다.
    // - space = 8 + 8: 계정 데이터 크기(바이트)
    //     8바이트: Anchor의 internal discriminator (타입 구분자)
    //     8바이트: Counter 구조체의 u64 필드(count)
    #[account(init, payer = user, space = 8 + 8)]
    pub counter: Account<'info, Counter>,

    // 트랜잭션을 서명하고, rent 비용을 지불하는 사용자
    // - mut: user의 lamport 잔액이 감소하므로 변경 가능해야 함
    #[account(mut)]
    pub user: Signer<'info>,

    // Solana 시스템 프로그램 (계정 생성에 필요)
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    // 이미 존재하는 counter 계정을 불러와 수정한다.
    // - mut: count 값을 증가시키므로 계정 데이터가 수정됨
    #[account(mut)]
    pub counter: Account<'info, Counter>,
}

// -----------------------------------------------------------------------------
// 4. Account 구조체 정의
// -----------------------------------------------------------------------------
// 실제 온체인에 저장될 데이터 구조체이다.
// Anchor는 이 구조체를 Borsh 포맷으로 직렬화/역직렬화한다.
// #[account] 매크로는 해당 구조체를 온체인 스토리지로 사용할 수 있게 한다.

#[account]
pub struct Counter {
    pub count: u64,
}
