// 저장된 Counter 주소를 사용해서 값을 증가시키는 스크립트
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import * as fs from "fs";
import * as path from "path";
import { MyCounter } from "../target/types/my_counter";

async function main() {
  // Provider 설정
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 프로그램 가져오기
  const program = anchor.workspace.myCounter as Program<MyCounter>;

  // 저장된 Counter 주소 읽기
  const deployedDataPath = path.join(__dirname, "../deployed-addresses.json");

  if (!fs.existsSync(deployedDataPath)) {
    console.error("ERROR: deployed-addresses.json 파일이 없습니다!");
    console.log("먼저 'anchor migrate'를 실행하세요.");
    process.exit(1);
  }

  const deployedData = JSON.parse(fs.readFileSync(deployedDataPath, "utf-8"));
  const counterAddress = new anchor.web3.PublicKey(deployedData.counterAddress);

  console.log("\nCounter 주소:", counterAddress.toString());

  // 현재 값 확인
  let counterData = await program.account.counter.fetch(counterAddress);
  console.log("현재 값:", counterData.count.toString());

  // Increment 3번 실행
  console.log("\nCounter 증가 중...");

  for (let i = 0; i < 3; i++) {
    const tx = await program.methods
      .increment()
      .accounts({
        counter: counterAddress,
      })
      .rpc();

    console.log(`Increment ${i + 1}/3 완료: ${tx.substring(0, 20)}...`);
  }

  // 최종 값 확인
  counterData = await program.account.counter.fetch(counterAddress);
  console.log("\n최종 값:", counterData.count.toString());
}

main()
  .then(() => {
    console.log("\n스크립트 완료!\n");
    process.exit(0);
  })
  .catch((error) => {
    console.error("\nERROR:", error);
    process.exit(1);
  });
