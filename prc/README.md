# Solana Anchor 학습 프로젝트

Solana 블록체인과 Anchor 프레임워크를 학습하기 위한 예제 프로젝트 모음입니다.

## 목차

- [Anchor란?](#anchor란)
- [Anchor 프로젝트 구조](#anchor-프로젝트-구조)
- [예시 프로젝트](#예시-프로젝트)
- [예제에서 사용된 문법 및 기능](#예제에서-사용된-문법-및-기능)

---

## Anchor란?

### Anchor 프레임워크

**Anchor**는 Solana 스마트 컨트랙트(프로그램) 개발을 쉽게 만들어주는 프레임워크입니다.

#### Anchor를 사용하는 이유

**Vanilla Solana (순수 Rust):**

```rust
// 계정 파싱, 검증 등을 모두 수동으로 작성
let accounts_iter = &mut accounts.iter();
let account1 = next_account_info(accounts_iter)?;
let account2 = next_account_info(accounts_iter)?;
// 계정 소유권 체크
if account1.owner != program_id {
    return Err(ProgramError::IncorrectProgramId);
}
// ... 수백 줄의 검증 코드
```

**Anchor:**

```rust
// 매크로가 자동으로 검증 코드 생성
#[derive(Accounts)]
pub struct MyAccounts<'info> {
    #[account(mut)]
    pub account1: Account<'info, MyData>,
    pub account2: Signer<'info>,
}
```

#### Anchor가 제공하는 것

- **자동 계정 검증**: `#[account]` 매크로로 안전성 향상
- **타입 안전성**: Rust 타입 시스템 활용
- **IDL 자동 생성**: TypeScript 클라이언트 자동 생성
- **테스트 프레임워크**: 쉬운 테스트 환경
- **에러 처리**: 명확한 에러 메시지

---

## Anchor 프로젝트 구조

### 기본 디렉토리 구조

```
my_project/
├── Anchor.toml          # Anchor 설정 파일
├── Cargo.toml           # Rust 워크스페이스 설정
├── package.json         # Node.js 의존성
├── programs/            # 스마트 컨트랙트 코드
│   └── my_project/
│       ├── Cargo.toml   # 프로그램 의존성
│       └── src/
│           └── lib.rs   # 메인 컨트랙트 코드
├── tests/               # TypeScript 테스트
│   └── my_project.ts
├── migrations/          # 배포 스크립트
│   └── deploy.ts
└── target/              # 빌드 결과물
    ├── deploy/          # 배포용 .so 파일
    ├── idl/             # IDL JSON 파일
    └── types/           # TypeScript 타입 정의
```

### 주요 파일 설명

#### 1. `Anchor.toml`

```toml
[programs.localnet]
my_project = "프로그램ID"

[provider]
cluster = "Localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha ..."
```

- 프로그램 ID와 네트워크 설정
- 지갑 경로
- 커스텀 스크립트 정의

#### 2. `programs/my_project/src/lib.rs`

```rust
use anchor_lang::prelude::*;

declare_id!("프로그램ID");

#[program]
pub mod my_project {
    use super::*;

    pub fn my_function(ctx: Context<MyContext>) -> Result<()> {
        // 비즈니스 로직
        Ok(())
    }
}

#[derive(Accounts)]
pub struct MyContext<'info> {
    // 계정 정의
}
```

- 메인 컨트랙트 코드
- `#[program]`: entrypoint 함수들 정의
- `#[derive(Accounts)]`: 계정 구조체 정의

#### 3. `tests/my_project.ts`

```typescript
import * as anchor from "@coral-xyz/anchor";

describe("my_project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.myProject;

  it("테스트", async () => {
    await program.methods.myFunction().rpc();
  });
});
```

- TypeScript로 작성된 테스트
- Anchor가 IDL로부터 자동 생성한 클라이언트 사용

---

## 예시 프로젝트

### 1. my_counter - 기본 구조 학습

#### 프로세스 흐름

```
1. 사용자가 initialize() 호출
   ↓
2. Counter 계정 생성 (count = 0)
   ↓
3. 사용자가 increment() 호출
   ↓
4. Counter.count += 1
```

#### 핵심 코드

```rust
#[program]
pub mod my_counter {
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.counter.count = 0;
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.counter.count += 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Counter {
    pub count: u64,
}
```

#### 학습 포인트

- `#[program]` 매크로: entrypoint 정의
- `Context<T>`: 계정과 메타데이터를 담는 컨테이너
- `#[account(init)]`: 새 계정 생성
- `Account<'info, T>`: 타입 안전한 계정 래퍼
- `Signer<'info>`: 트랜잭션 서명자

**실행:**

```bash
cd my_counter
anchor test
```

---

### 2. voting - PDA와 복잡한 로직

#### 프로세스 흐름

```
1. 관리자가 initialize_poll() 호출
   ↓
2. Poll 계정 생성 (PDA: seeds=[poll_id])
   - poll_id, 설명, 시작/종료 시간 저장
   ↓
3. 관리자가 initialize_candidate() 호출 (여러 번)
   ↓
4. Candidate 계정 생성 (PDA: seeds=[poll_id, name])
   - Poll.candidate_amount 증가
   ↓
5. 사용자들이 vote() 호출
   ↓
6. 시간 체크 후 Candidate.votes 증가
```

#### 핵심 코드

```rust
#[program]
pub mod voting {
    pub fn initialize_poll(
        ctx: Context<InitializePoll>,
        poll_id: u64,
        description: String,
        poll_start: u64,
        poll_end: u64,
    ) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        require!(poll_start < poll_end, VotingError::InvalidTimeRange);

        poll.poll_id = poll_id;
        poll.description = description;
        poll.poll_start = poll_start;
        poll.poll_end = poll_end;
        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, ...) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp as u64;
        require!(
            current_time >= poll.poll_start,
            VotingError::PollNotStarted
        );

        ctx.accounts.candidate.candidate_votes += 1;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(poll_id: u64)]
pub struct InitializePoll<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + Poll::INIT_SPACE,
        seeds = [poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll: Account<'info, Poll>,
    // ...
}
```

#### 학습 포인트

- **PDA (Program Derived Address)**: 결정적 계정 주소
  - `seeds = [poll_id.to_le_bytes().as_ref()]`
  - 프라이빗 키 없이 프로그램이 서명 가능
- `#[instruction(...)]`: 함수 파라미터를 Context에서 사용
- `require!()`: 조건 검증 매크로
- `Clock::get()`: 온체인 시간 가져오기
- `#[error_code]`: 커스텀 에러 정의
- `#[derive(InitSpace)]`: 계정 크기 자동 계산

**실행:**

```bash
cd voting
anchor test
```

---

### 3. token_vault - CPI와 외부 프로그램 연동

#### 프로세스 흐름

```
1. 관리자가 initialize_vault() 호출
   ↓
2. Vault PDA 생성 (seeds=["vault"])
   - Vault Token Account 주소 저장
   ↓
3. 사용자가 initialize_user_vault() 호출
   ↓
4. UserVault PDA 생성 (seeds=["user-vault", user, vault])
   - deposited_amount = 0
   ↓
5. 사용자가 deposit(amount) 호출
   ↓
6. CPI로 Token Program 호출
   - 사용자 Token Account → Vault Token Account
   ↓
7. UserVault.deposited_amount += amount
   ↓
8. 사용자가 withdraw(amount) 호출
   ↓
9. 잔액 체크 후 CPI로 Token Program 호출 (Vault PDA 서명)
   - Vault Token Account → 사용자 Token Account
   ↓
10. UserVault.deposited_amount -= amount
```

#### 핵심 코드

```rust
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[program]
pub mod token_vault {
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // CPI: Token Program 호출
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

        ctx.accounts.user_vault_account.deposited_amount += amount;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // PDA 서명으로 CPI 호출
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

        ctx.accounts.user_vault_account.deposited_amount -= amount;
        Ok(())
    }
}
```

#### 학습 포인트

- **CPI (Cross Program Invocation)**: 다른 프로그램 호출
  - `CpiContext::new()`: 일반 CPI
  - `CpiContext::new_with_signer()`: PDA 서명 CPI
- **SPL Token Program**: Solana 표준 토큰
  - Token Account: 토큰을 담는 계정
  - Mint: 토큰 종류 정의
- **PDA 서명**:
  - `seeds`와 `bump`로 PDA 생성
  - 프로그램이 PDA 대신 서명 가능
- `anchor_spl`: SPL 프로그램 Anchor 래퍼

**실행:**

```bash
cd token_vault
anchor test
```

---

## 예제에서 사용된 문법 및 기능

### 1. 기본 Anchor 매크로

#### `#[program]`

```rust
#[program]
pub mod my_program {
    use super::*;

    // 이 함수들이 entrypoint가 됨
    pub fn my_function(ctx: Context<MyCtx>) -> Result<()> {
        Ok(())
    }
}
```

- 프로그램의 instruction handler 정의
- 각 함수가 온체인에서 호출 가능한 instruction이 됨

#### `#[derive(Accounts)]`

```rust
#[derive(Accounts)]
pub struct MyContext<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyData>,
    pub signer: Signer<'info>,
}
```

- 계정 구조체 정의
- Anchor가 자동으로 파싱/검증 코드 생성
- `<'info>`: Rust 라이프타임 (계정 참조가 유효한 범위)

#### `#[account]`

```rust
#[account]
pub struct MyData {
    pub value: u64,
    pub owner: Pubkey,
}
```

- 온체인에 저장될 데이터 구조 정의
- 자동으로 직렬화/역직렬화

### 2. Account 타입

| 타입                   | 설명               | 예시                               |
| ---------------------- | ------------------ | ---------------------------------- |
| `Account<'info, T>`    | 데이터를 담는 계정 | `Account<'info, Counter>`          |
| `Signer<'info>`        | 트랜잭션 서명자    | `pub user: Signer<'info>`          |
| `Program<'info, T>`    | 다른 프로그램 참조 | `Program<'info, System>`           |
| `SystemAccount<'info>` | 일반 SOL 계정      | `pub wallet: SystemAccount<'info>` |

### 3. Account 제약 조건

```rust
#[derive(Accounts)]
pub struct MyAccounts<'info> {
    // 새 계정 생성
    #[account(
        init,
        payer = user,
        space = 8 + 8
    )]
    pub new_account: Account<'info, MyData>,

    // 수정 가능
    #[account(mut)]
    pub mutable_account: Account<'info, MyData>,

    // PDA
    #[account(
        seeds = [b"my-seed", user.key().as_ref()],
        bump
    )]
    pub pda_account: Account<'info, MyData>,

    // 조건 검증
    #[account(
        constraint = my_account.owner == user.key()
    )]
    pub my_account: Account<'info, MyData>,
}
```

### 4. Context<T>

```rust
pub fn my_function(ctx: Context<MyAccounts>) -> Result<()> {
    // ctx.accounts: 계정들에 접근
    let data = &mut ctx.accounts.my_account;

    // ctx.bumps: PDA bump 값들
    let bump = ctx.bumps.pda_account;

    // ctx.program_id: 현재 프로그램 ID
    msg!("Program ID: {}", ctx.program_id);

    Ok(())
}
```

- `Context<T>`는 계정, bump, program_id 등을 담는 컨테이너
- DTO(Data Transfer Object)와 유사한 역할

### 5. PDA (Program Derived Address)

```rust
// 컨트랙트에서
#[account(
    seeds = [b"my-seed", user.key().as_ref()],
    bump
)]
pub my_pda: Account<'info, MyData>

// 클라이언트에서 (TypeScript)
const [pdaAddress, bump] = PublicKey.findProgramAddressSync(
  [Buffer.from("my-seed"), user.publicKey.toBuffer()],
  programId
);
```

- **결정적 주소**: seeds로부터 항상 같은 주소 생성
- **프라이빗 키 없음**: 프로그램만 서명 가능
- **용도**: 사용자별 데이터, 프로그램 소유 계정

### 6. 기본 Rust 문법

```rust
// Result 타입: 성공(Ok) 또는 에러(Err)
pub fn my_function() -> Result<()> {
    // ? 연산자: 에러 자동 전파
    some_function()?;
    Ok(())
}

// 참조
let x = 5;
let y = &x;      // 불변 참조
let z = &mut x;  // 가변 참조

// 구조체 메서드
impl MyStruct {
    pub fn new() -> Self {
        Self { value: 0 }
    }
}

// 열거형
pub enum MyEnum {
    Option1,
    Option2(u64),
}
```

### 7. 에러 처리

```rust
#[error_code]
pub enum MyError {
    #[msg("에러 메시지")]
    MyError,
}

// 사용
require!(condition, MyError::MyError);

// 또는
if !condition {
    return err!(MyError::MyError);
}
```

---

# 예제별 테스트 실행하기

해당 디렉토리로 이동 후

```
anchor test
anchor test --skip-local-validator // 해당 명령어는 로컬에 실행시켜놓은 노드로 테스트 진행함
```

---

# 자주 사용하는 Anchor CLI 커맨드

## 프로젝트 관리

```bash
anchor init <project-name>   # 새 프로젝트 생성
anchor clean                 # 빌드 아티팩트 정리 (target/ 삭제)
```

## 빌드 & 배포

```bash
anchor build                        # 프로그램 빌드 (IDL 생성 포함)
anchor deploy                       # 프로그램을 체인에 배포
anchor migrate                      # 배포 + migrations/ 스크립트 실행
anchor idl init <program-id>        # IDL 초기화
anchor idl upgrade <program-id>     # IDL 업그레이드
anchor idl fetch <program-id>       # IDL 가져오기
```

## 테스트

```bash
anchor test                         # 테스트 실행 (로컬 validator 자동 시작/종료)
anchor test --skip-local-validator  # 이미 실행 중인 validator 사용
anchor test --skip-build            # 빌드 스킵하고 테스트만
```

## 로컬 개발

```bash
anchor localnet                     # 로컬 validator 시작
anchor run <script-name>            # Anchor.toml의 커스텀 스크립트 실행
```

## 키 & 계정 관리

```bash
anchor keys list                    # 프로그램 키 조회
anchor keys sync                    # declare_id!()와 Anchor.toml의 program_id 동기화
```

## 디버깅

```bash
anchor expand                       # 매크로 확장 코드 보기 (Anchor 매크로가 실제로 뭘 하는지)
```

## 기타

```bash
anchor upgrade <program-id>         # 배포된 프로그램 업그레이드
anchor shell                        # Anchor 개발 환경 셸 진입
```
