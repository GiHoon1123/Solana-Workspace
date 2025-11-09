import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import { Voting } from "../target/types/voting";

describe("voting", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.voting as Program<Voting>;

  // 테스트용 데이터
  const pollId = new BN(Math.floor(Math.random() * 1000000)); // 랜덤 ID로 충돌 방지
  const pollDescription = "대통령 선거";
  const candidateA = "후보A";
  const candidateB = "후보B";

  // PDA 주소 계산
  let pollAddress: PublicKey;
  let candidateAAddress: PublicKey;
  let candidateBAddress: PublicKey;

  before(async () => {
    // Poll PDA
    [pollAddress] = PublicKey.findProgramAddressSync(
      [pollId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // Candidate A PDA
    [candidateAAddress] = PublicKey.findProgramAddressSync(
      [pollId.toArrayLike(Buffer, "le", 8), Buffer.from(candidateA)],
      program.programId
    );

    // Candidate B PDA
    [candidateBAddress] = PublicKey.findProgramAddressSync(
      [pollId.toArrayLike(Buffer, "le", 8), Buffer.from(candidateB)],
      program.programId
    );

    console.log("\nPDA 주소:");
    console.log("Poll:", pollAddress.toString());
    console.log("Candidate A:", candidateAAddress.toString());
    console.log("Candidate B:", candidateBAddress.toString());
  });

  it("투표를 생성한다", async () => {
    const now = Math.floor(Date.now() / 1000);
    const pollStart = new BN(now - 10); // 10초 전부터 시작
    const pollEnd = new BN(now + 3600); // 1시간 후 종료

    const tx = await program.methods
      .initializePoll(pollId, pollDescription, pollStart, pollEnd)
      .accounts({
        signer: provider.wallet.publicKey,
        poll: pollAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\n투표 생성 트랜잭션:", tx);

    // Poll 계정 데이터 확인
    const pollAccount = await program.account.poll.fetch(pollAddress);

    assert.equal(pollAccount.pollId.toNumber(), pollId.toNumber());
    assert.equal(pollAccount.description, pollDescription);
    assert.equal(pollAccount.candidateAmount.toNumber(), 0);
    console.log("투표 생성 성공:", pollAccount.description);
  });

  it("후보자 A를 추가한다", async () => {
    const tx = await program.methods
      .initializeCandidate(candidateA, pollId)
      .accounts({
        signer: provider.wallet.publicKey,
        poll: pollAddress,
        candidate: candidateAAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\n후보자 A 추가 트랜잭션:", tx);

    // Candidate 계정 확인
    const candidateAccount = await program.account.candidate.fetch(
      candidateAAddress
    );
    assert.equal(candidateAccount.candidateName, candidateA);
    assert.equal(candidateAccount.candidateVotes.toNumber(), 0);

    // Poll의 후보자 수 확인
    const pollAccount = await program.account.poll.fetch(pollAddress);
    assert.equal(pollAccount.candidateAmount.toNumber(), 1);

    console.log("후보자 A 추가 성공:", candidateAccount.candidateName);
  });

  it("후보자 B를 추가한다", async () => {
    const tx = await program.methods
      .initializeCandidate(candidateB, pollId)
      .accounts({
        signer: provider.wallet.publicKey,
        poll: pollAddress,
        candidate: candidateBAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\n후보자 B 추가 트랜잭션:", tx);

    const candidateAccount = await program.account.candidate.fetch(
      candidateBAddress
    );
    assert.equal(candidateAccount.candidateName, candidateB);
    assert.equal(candidateAccount.candidateVotes.toNumber(), 0);

    const pollAccount = await program.account.poll.fetch(pollAddress);
    assert.equal(pollAccount.candidateAmount.toNumber(), 2);

    console.log("후보자 B 추가 성공:", candidateAccount.candidateName);
  });

  it("후보자 A에게 투표한다", async () => {
    const tx = await program.methods
      .vote(candidateA, pollId)
      .accounts({
        signer: provider.wallet.publicKey,
        poll: pollAddress,
        candidate: candidateAAddress,
      })
      .rpc();

    console.log("\n후보자 A 투표 트랜잭션:", tx);

    const candidateAccount = await program.account.candidate.fetch(
      candidateAAddress
    );
    assert.equal(candidateAccount.candidateVotes.toNumber(), 1);

    console.log("후보자 A 득표수:", candidateAccount.candidateVotes.toNumber());
  });

  it("후보자 B에게 투표한다", async () => {
    const tx = await program.methods
      .vote(candidateB, pollId)
      .accounts({
        signer: provider.wallet.publicKey,
        poll: pollAddress,
        candidate: candidateBAddress,
      })
      .rpc();

    console.log("\n후보자 B 투표 트랜잭션:", tx);

    const candidateAccount = await program.account.candidate.fetch(
      candidateBAddress
    );
    assert.equal(candidateAccount.candidateVotes.toNumber(), 1);

    console.log("후보자 B 득표수:", candidateAccount.candidateVotes.toNumber());
  });

  it("후보자 A에게 여러 번 투표한다", async () => {
    // 3번 더 투표
    for (let i = 0; i < 3; i++) {
      await program.methods
        .vote(candidateA, pollId)
        .accounts({
          signer: provider.wallet.publicKey,
          poll: pollAddress,
          candidate: candidateAAddress,
        })
        .rpc();
    }

    const candidateAccount = await program.account.candidate.fetch(
      candidateAAddress
    );
    assert.equal(candidateAccount.candidateVotes.toNumber(), 4); // 1 + 3 = 4

    console.log(
      "\n후보자 A 최종 득표수:",
      candidateAccount.candidateVotes.toNumber()
    );
  });

  it("최종 결과를 확인한다", async () => {
    console.log("\n=== 투표 결과 ===");

    const pollAccount = await program.account.poll.fetch(pollAddress);
    console.log("투표 제목:", pollAccount.description);
    console.log("총 후보자 수:", pollAccount.candidateAmount.toNumber());

    const candidateAAccount = await program.account.candidate.fetch(
      candidateAAddress
    );
    console.log(
      `${
        candidateAAccount.candidateName
      }: ${candidateAAccount.candidateVotes.toNumber()}표`
    );

    const candidateBAccount = await program.account.candidate.fetch(
      candidateBAddress
    );
    console.log(
      `${
        candidateBAccount.candidateName
      }: ${candidateBAccount.candidateVotes.toNumber()}표`
    );

    // 결과 검증
    assert.equal(candidateAAccount.candidateVotes.toNumber(), 4);
    assert.equal(candidateBAccount.candidateVotes.toNumber(), 1);
  });
});
