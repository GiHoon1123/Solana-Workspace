// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import * as fs from "fs";
import * as path from "path";
import { MyCounter } from "../target/types/my_counter";

module.exports = async function (provider: anchor.AnchorProvider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  console.log("\n======================================");
  console.log("배포 후 초기화 스크립트 시작");
  console.log("Wallet:", provider.wallet.publicKey.toString());
  console.log("======================================\n");

  // 프로그램 가져오기
  const program = anchor.workspace.myCounter as Program<MyCounter>;
  console.log("프로그램 ID:", program.programId.toString());

  // Counter 계정 생성
  const counterAccount = anchor.web3.Keypair.generate();
  console.log("\n새 Counter 계정 생성:");
  console.log("주소:", counterAccount.publicKey.toString());

  try {
    // initialize 함수 호출
    console.log("\nCounter 초기화 중...");
    const tx = await program.methods
      .initialize()
      .accounts({
        counter: counterAccount.publicKey,
        user: provider.wallet.publicKey,
        // systemProgram은 자동으로 추론됨
      })
      .signers([counterAccount])
      .rpc();

    console.log("초기화 성공!");
    console.log("트랜잭션:", tx);

    // 계정 데이터 확인
    const counterData = await program.account.counter.fetch(
      counterAccount.publicKey
    );
    console.log("\n생성된 Counter 정보:");
    console.log("현재 값:", counterData.count.toString());

    // 계정 주소를 파일로 저장 (나중에 재사용 가능)
    const deployedData = {
      programId: program.programId.toString(),
      counterAddress: counterAccount.publicKey.toString(),
      deployedAt: new Date().toISOString(),
      network: provider.connection.rpcEndpoint,
    };

    const outputPath = path.join(__dirname, "../deployed-addresses.json");
    fs.writeFileSync(outputPath, JSON.stringify(deployedData, null, 2));
    console.log("\n배포 정보 저장됨:");
    console.log("파일:", outputPath);

    console.log("\n======================================");
    console.log("배포 및 초기화 완료!");
    console.log("======================================\n");
  } catch (error) {
    console.error("\n에러 발생:", error);
    throw error;
  }
};
