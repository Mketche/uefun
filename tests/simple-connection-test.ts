import * as anchor from "@coral-xyz/anchor";
import { Connection } from '@solana/web3.js';

describe("测试Solana连接", () => {
  // 创建一个连接到本地测试验证器的连接
  const connection = new Connection("http://localhost:8899", "confirmed");
  
  it("可以连接到Solana网络", async () => {
    // 检查连接是否有效
    const version = await connection.getVersion();
    console.log("Solana版本:", version);
    console.log("连接成功!");
  });
}); 