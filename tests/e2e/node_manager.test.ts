import { muta, admin as ADMIN, delay, mutaClient as client, accounts } from "./utils";
import { add_fee_token_to_accounts } from "./helper";
import { hexToNum } from "@mutajs/utils";

async function setAdmin(admin) {
  const tx = await client.composeTransaction({
    method: "set_admin",
    payload: {
      admin
    },
    serviceName: "node_manager"
  });
  const signed_tx = ADMIN.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  console.log(receipt);
  return receipt;
}

async function getAdmin() {
  const res = await client.queryService({
    serviceName: "node_manager",
    method: "get_admin",
    payload: ""
  });
  return res;
}

async function updateInterval(admin, interval) {
  const tx = await client.composeTransaction({
    method: "update_interval",
    payload: {
      interval
    },
    serviceName: "node_manager"
  });
  const signed_tx = admin.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

async function updateRatio(
  admin,
  propose_ratio,
  prevote_ratio,
  precommit_ratio,
  brake_ratio
) {
  const tx = await client.composeTransaction({
    method: "update_ratio",
    payload: {
      propose_ratio,
      prevote_ratio,
      precommit_ratio,
      brake_ratio
    },
    serviceName: "node_manager"
  });
  const signed_tx = admin.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

async function getMetadata() {
  const res = await client.queryService({
    serviceName: "metadata",
    method: "get_metadata",
    payload: ""
  });
  return res;
}

describe("node manager service API test via muta-sdk-js", () => {
  beforeAll(async () => {
    await add_fee_token_to_accounts(accounts.map(a => a.address));
  });

  test("test regular progress", async () => {
    // Set admin
    let receipt = await setAdmin(accounts[0].address);
    expect(hexToNum(receipt.response.response.code)).toBe(0);

    // Get admin
    let res = await getAdmin();
    expect(hexToNum(res.code)).toBe(0);

    let admin_addr = JSON.parse(res.succeedData);
    expect(admin_addr).toBe(accounts[0].address);

    // Update interval
    let admin = accounts[0];
    receipt = await updateInterval(admin, 666);
    expect(hexToNum(receipt.response.response.code)).toBe(0);

    res = await getMetadata();
    let metadata = JSON.parse(res.succeedData);
    expect(hexToNum(metadata.interval)).toBe(666);

    // Update ratio
    receipt = await updateRatio(admin, 16, 16, 16, 6);
    expect(hexToNum(receipt.response.response.code)).toBe(0);

    res = await getMetadata();
    metadata = JSON.parse(res.succeedData);

    expect(metadata.propose_ratio).toBe(16);
    expect(metadata.prevote_ratio).toBe(16);
    expect(metadata.precommit_ratio).toBe(16);
    expect(metadata.brake_ratio).toBe(6);
  });
});
