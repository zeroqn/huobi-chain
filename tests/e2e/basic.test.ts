import { muta, CHAIN_CONFIG, delay, mutaClient as client, accounts } from "./utils";
import { hexToNum } from "@mutajs/utils";

describe("basic API test via muta-sdk-js", () => {
  test("getLatestBlockHeight", async () => {
    const current_height = await client.getLatestBlockHeight();
    // console.log(current_height);
    expect(current_height).toBeGreaterThan(0);
  });

  test("getBlock", async () => {
    const block = await client.getBlock("0x01");
    // console.log(block);
    expect(hexToNum(block.header.height)).toBe(1);
  });

  test("send_tx_exceed_cycles_limit", async () => {
    const tx = await client.composeTransaction({
      method: "create_asset",
      payload: {
        name: "Muta Token",
        symbol: "MT",
        supply: 1000000000
      },
      serviceName: "asset"
    });
    tx.cyclesLimit = "0xE8D4A51FFF";
    const account = accounts[0];
    const signed_tx = account.signTransaction(tx);
    // console.log(signed_tx);
    try {
      const hash = await client.sendTransaction(signed_tx);
      expect(true).toBe(false);
    } catch (err) {
      // console.log(err);
      expect(err.response.errors[0].message.includes("ExceedCyclesLimit")).toBe(
        true
      );
    }
  });

  test("send_tx_exceed_tx_size_limit", async () => {
    const tx = await client.composeTransaction({
      method: "create_asset",
      payload: {
        name: "Muta Token",
        symbol: "MT",
        supply: 1000000000,
        bigdata: "a".repeat(300000)
      },
      serviceName: "asset"
    });

    const account = accounts[0];
    const signed_tx = account.signTransaction(tx);

    try {
      await client.sendTransaction(signed_tx);
    } catch (err) {
      const err_msg = err.response.errors[0].message;
      expect(err_msg.includes("ExceedSizeLimit")).toBe(true);
    }
  });

  test("send tx, get tx and receipt", async () => {
    const tx = await client.composeTransaction({
      method: "create_asset",
      payload: {
        name: "Muta Token",
        symbol: "MT",
        supply: 1000000000
      },
      serviceName: "asset"
    });

    const account = accounts[0];
    const signed_tx = account.signTransaction(tx);

    const hash = await client.sendTransaction(signed_tx);
    const receipt = await client.getReceipt(hash);
    expect(receipt.txHash).toBe(hash);

    const committed_tx = await client.getTransaction(hash);
    expect(committed_tx.txHash).toBe(hash);
  });
});
