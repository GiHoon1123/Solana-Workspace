import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { MyCounter } from "../target/types/my_counter";

describe("my_counter", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.myCounter as Program<MyCounter>;
  const provider = anchor.AnchorProvider.env();

  // Counter 계정을 위한 새 Keypair 생성
  const counterAccount = anchor.web3.Keypair.generate();

  it("Is initialized!", async () => {
    // initialize 함수 호출 - counter 계정 생성 및 초기화
    const tx = await program.methods
      .initialize()
      .accounts({
        counter: counterAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([counterAccount])
      .rpc();

    console.log("Initialize transaction signature:", tx);

    // 계정 데이터 읽어오기
    const counterData = await program.account.counter.fetch(
      counterAccount.publicKey
    );

    console.log("📊 Counter value:", counterData.count.toString());

    // count가 0인지 확인
    expect(counterData.count.toNumber()).to.equal(0);
  });

  it("Increments the counter!", async () => {
    // increment 함수 호출 - count 값 증가
    const tx = await program.methods
      .increment()
      .accounts({
        counter: counterAccount.publicKey,
      })
      .rpc();

    console.log("Increment transaction signature:", tx);

    // 계정 데이터 다시 읽어오기
    const counterData = await program.account.counter.fetch(
      counterAccount.publicKey
    );

    console.log("Counter value after increment:", counterData.count.toString());

    // count가 1인지 확인
    expect(counterData.count.toNumber()).to.equal(1);
  });

  it("Increments the counter multiple times!", async () => {
    // 여러 번 증가시키기
    await program.methods
      .increment()
      .accounts({
        counter: counterAccount.publicKey,
      })
      .rpc();

    await program.methods
      .increment()
      .accounts({
        counter: counterAccount.publicKey,
      })
      .rpc();

    // 계정 데이터 확인
    const counterData = await program.account.counter.fetch(
      counterAccount.publicKey
    );

    console.log("Final counter value:", counterData.count.toString());

    // count가 3인지 확인 (1 + 1 + 1 = 3)
    expect(counterData.count.toNumber()).to.equal(3);
  });
});
