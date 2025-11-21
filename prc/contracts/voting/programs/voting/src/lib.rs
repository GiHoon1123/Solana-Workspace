use anchor_lang::prelude::*;

declare_id!("Bk15d8TNtEorYDBXLtjRjSt1hB5tEwe3MsGcHdwAjq7b");

// -----------------------------------------------------------------------------
// 프로그램 모듈
// -----------------------------------------------------------------------------
// #[program]: 이 모듈의 함수들을 온체인 entrypoint로 변환
#[program]
pub mod voting {
    use super::*;

    // 투표 생성
    pub fn initialize_poll(
        ctx: Context<InitializePoll>,
        poll_id: u64,
        description: String,
        poll_start: u64,
        poll_end: u64,
    ) -> Result<()> {
        let poll = &mut ctx.accounts.poll;

        // 입력 검증
        require!(description.len() <= 280, VotingError::DescriptionTooLong);
        require!(poll_start < poll_end, VotingError::InvalidTimeRange);

        poll.poll_id = poll_id;
        poll.description = description;
        poll.poll_start = poll_start;
        poll.poll_end = poll_end;
        poll.candidate_amount = 0;
        poll.creator = ctx.accounts.signer.key();

        Ok(())
    }

    // 후보자 추가
    pub fn initialize_candidate(
        ctx: Context<InitializeCandidate>,
        candidate_name: String,
        _poll_id: u64,
    ) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        let candidate = &mut ctx.accounts.candidate;

        // 입력 검증
        require!(candidate_name.len() <= 50, VotingError::CandidateNameTooLong);
        require!(poll.candidate_amount < 5, VotingError::TooManyCandidates);

        // 후보자 정보 설정
        candidate.candidate_name = candidate_name;
        candidate.candidate_votes = 0;

        // Poll의 후보자 수 증가
        poll.candidate_amount += 1;

        Ok(())
    }

    // 투표하기
    pub fn vote(
        ctx: Context<Vote>,
        _candidate_name: String,
        _poll_id: u64,
    ) -> Result<()> {
        let poll = &ctx.accounts.poll;
        let candidate = &mut ctx.accounts.candidate;

        // 투표 기간 검증
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(
            current_time >= poll.poll_start,
            VotingError::PollNotStarted
        );
        require!(
            current_time <= poll.poll_end,
            VotingError::PollEnded
        );

        // 투표 카운트 증가
        candidate.candidate_votes += 1;

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Context 구조체들
// -----------------------------------------------------------------------------

// 투표 생성
// #[derive(Accounts)]: Anchor가 계정 파싱 및 검증 코드 자동 생성
#[derive(Accounts)]
// #[instruction(...)]: 함수 파라미터를 Context에서 사용 가능하게 함
#[instruction(poll_id: u64)]
pub struct InitializePoll<'info> {
    // #[account(mut)]: 이 계정은 수정 가능 (lamports 차감됨)
    #[account(mut)]
    pub signer: Signer<'info>,

    // #[account(...)]: 계정 제약 조건 정의
    #[account(
        init,                              // 새 계정 생성
        payer = signer,                    // signer가 계정 생성 비용 지불
        space = 8 + Poll::INIT_SPACE,      // 계정 크기 (8 = discriminator)
        seeds = [poll_id.to_le_bytes().as_ref()],  // PDA seeds
        bump                               // PDA bump (자동 계산)
    )]
    pub poll: Account<'info, Poll>,

    pub system_program: Program<'info, System>,
}

// 후보자 추가
#[derive(Accounts)]
#[instruction(candidate_name: String, poll_id: u64)]
pub struct InitializeCandidate<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    // mut: 기존 계정을 수정 가능하게 (candidate_amount 증가)
    #[account(
        mut,                              // Poll 계정의 데이터 수정 가능
        seeds = [poll_id.to_le_bytes().as_ref()],  // PDA 검증
        bump
    )]
    pub poll: Account<'info, Poll>,

    // init: 새 Candidate 계정 생성
    #[account(
        init,
        payer = signer,
        space = 8 + Candidate::INIT_SPACE,
        seeds = [poll_id.to_le_bytes().as_ref(), candidate_name.as_bytes()],  // poll_id + 이름
        bump
    )]
    pub candidate: Account<'info, Candidate>,

    pub system_program: Program<'info, System>,
}

// 투표하기
#[derive(Accounts)]
#[instruction(candidate_name: String, poll_id: u64)]
pub struct Vote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    // seeds만 있고 mut 없음: 읽기 전용으로 PDA 검증만 수행
    #[account(
        seeds = [poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll: Account<'info, Poll>,

    // mut: 득표수 증가를 위해 수정 가능
    #[account(
        mut,
        seeds = [poll_id.to_le_bytes().as_ref(), candidate_name.as_bytes()],
        bump
    )]
    pub candidate: Account<'info, Candidate>,
}

// -----------------------------------------------------------------------------
// 계정 데이터 구조체
// -----------------------------------------------------------------------------

// 투표 계정
// #[account]: 이 구조체를 온체인 계정 데이터로 사용 (직렬화/역직렬화 자동)
#[account]
// #[derive(InitSpace)]: 계정 크기 자동 계산 (INIT_SPACE 상수 생성)
#[derive(InitSpace)]
pub struct Poll {
    pub poll_id: u64,
    // #[max_len(...)]: String 최대 길이 지정 (공간 계산에 필요)
    #[max_len(280)]
    pub description: String,
    pub poll_start: u64,
    pub poll_end: u64,
    pub candidate_amount: u64,
    pub creator: Pubkey,
}

// 후보자 계정
#[account]
#[derive(InitSpace)]
pub struct Candidate {
    #[max_len(50)]
    pub candidate_name: String,
    pub candidate_votes: u64,
}

// -----------------------------------------------------------------------------
// 에러 정의
// -----------------------------------------------------------------------------
// #[error_code]: 에러 enum을 Anchor 에러로 변환 (자동 에러 코드 생성)
#[error_code]
pub enum VotingError {
    // #[msg(...)]: 에러 발생 시 표시될 메시지
    #[msg("설명이 너무 깁니다 (최대 280자)")]
    DescriptionTooLong,

    #[msg("후보자 이름이 너무 깁니다 (최대 50자)")]
    CandidateNameTooLong,

    #[msg("시작 시간이 종료 시간보다 늦습니다")]
    InvalidTimeRange,

    #[msg("투표가 아직 시작되지 않았습니다")]
    PollNotStarted,

    #[msg("투표가 종료되었습니다")]
    PollEnded,

    #[msg("최대 5명의 후보자만 추가할 수 있습니다")]
    TooManyCandidates,
}
