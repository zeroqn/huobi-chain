import {
  muta,
  CHAIN_CONFIG,
  delay,
  mutaClient as client,
  accounts,
  admin,
  fee_asset_id,
  fee_account
} from "./utils";
import { hexToNum } from "@mutajs/utils";

async function createAsset(txSender, name, symbol, supply, precision) {
  const payload = {
    name,
    symbol,
    supply,
    precision
  };
  const tx = await client.composeTransaction({
    method: "create_asset",
    payload,
    serviceName: "asset"
  });
  const signed_tx = txSender.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

async function getAsset(assetID) {
  const res = await client.queryService({
    serviceName: "asset",
    method: "get_asset",
    payload: JSON.stringify({
      id: assetID
    })
  });
  return res;
}

async function transfer(txSender, assetID, to, value) {
  const payload = {
    asset_id: assetID,
    to,
    value
  };

  const tx = await client.composeTransaction({
    method: "transfer",
    payload,
    serviceName: "asset"
  });
  const signed_tx = txSender.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

async function getBalance(assetID, user) {
  const res = await client.queryService({
    serviceName: "asset",
    method: "get_balance",
    payload: JSON.stringify({
      asset_id: assetID,
      user: user
    })
  });
  return res;
}

async function approve(txSender, assetID, to, value) {
  const payload = {
    asset_id: assetID,
    to,
    value
  };

  const tx = await client.composeTransaction({
    method: "approve",
    payload,
    serviceName: "asset"
  });
  const signed_tx = txSender.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

async function getAllowance(assetID, grantor, grantee) {
  const res = await client.queryService({
    serviceName: "asset",
    method: "get_allowance",
    payload: JSON.stringify({
      asset_id: assetID,
      grantor,
      grantee
    })
  });
  return res;
}

async function transferFrom(txSender, assetID, sender, recipient, value) {
  const payload = {
    asset_id: assetID,
    sender,
    recipient,
    value
  };

  const tx = await client.composeTransaction({
    method: "transfer_from",
    payload,
    serviceName: "asset"
  });
  const signed_tx = txSender.signTransaction(tx);
  const hash = await client.sendTransaction(signed_tx);
  const receipt = await client.getReceipt(hash);
  return receipt;
}

describe("asset service API test via muta-sdk-js", () => {
  test("test normal process", async () => {
    // fee not enough
    // let caReceipt = await createAsset(
    //   accounts[0],
    //   "Test Token",
    //   "TT",
    //   8888,
    //   10000
    // );
    // expect(caReceipt.response.response.errorMessage).toBe("Lack of balance");

    // add fee token to accounts
    await Promise.all(
      accounts.map(account =>
        transfer(admin, fee_asset_id, account.address, 10000)
      )
    );

    // Create asset
    const fee_account_balance_before = await getBalance(
      fee_asset_id,
      fee_account
    );
    let caReceipt = await createAsset(accounts[0], "Test Token", "TT", 8888, 10000);
    expect(hexToNum(caReceipt.response.response.code)).toBe(0);
    const fee_account_balance_after = await getBalance(
      fee_asset_id,
      fee_account
    );
    const caRet = JSON.parse(caReceipt.response.response.succeedData);
    const assetID = caRet.id;

    // check fee account balance
    // FIXME: fee
    // expect(
    //   JSON.parse(fee_account_balance_before.succeedData).balance <
    //     JSON.parse(fee_account_balance_after.succeedData).balance
    // ).toBe(true);

    // Get asset
    const gaRes = await getAsset(assetID);
    const gaRet = JSON.parse(gaRes.succeedData);
    expect(gaRet.id).toBe(assetID);
    expect(gaRet.name).toBe("Test Token");
    expect(gaRet.symbol).toBe("TT");
    expect(gaRet.supply).toBe(8888);
    expect(gaRet.precision).toBe(10000);
    expect(gaRet.issuer).toBe(accounts[0].address);

    // Transfer
    const tranReceipt = await transfer(
      accounts[0],
      assetID,
      accounts[1].address,
      88
    );
    // console.log("transfer receipt: ", tranReceipt);
    expect(hexToNum(tranReceipt.response.response.code)).toBe(0);

    // Check balance
    const issuerBalanceRes = await getBalance(assetID, accounts[0].address);
    // console.log("balance res:", issuerBalanceRes);
    const issuerBalance = JSON.parse(issuerBalanceRes.succeedData).balance;
    let recipientBalanceRes = await getBalance(assetID, accounts[1].address);
    let recipientBalance = JSON.parse(recipientBalanceRes.succeedData).balance;
    expect(issuerBalance).toBe(8800);
    expect(recipientBalance).toBe(88);

    // Approve
    const apprReceipt = await approve(
      accounts[1],
      assetID,
      accounts[2].address,
      8
    );
    expect(hexToNum(apprReceipt.response.response.code)).toBe(0);

    // Check allowance
    let alloRes = await getAllowance(
      assetID,
      accounts[1].address,
      accounts[2].address
    );
    let allowance = JSON.parse(alloRes.succeedData).value;
    expect(allowance).toBe(8);

    // Transfer from
    const tfReceipt = await transferFrom(
      accounts[2],
      assetID,
      accounts[1].address,
      accounts[2].address,
      8
    );
    expect(hexToNum(tfReceipt.response.response.code)).toBe(0);

    // Check balance and allowance
    const senderBalanceRes = await getBalance(assetID, accounts[1].address);
    const senderBalance = JSON.parse(senderBalanceRes.succeedData).balance;
    recipientBalanceRes = await getBalance(assetID, accounts[2].address);
    recipientBalance = JSON.parse(recipientBalanceRes.succeedData).balance;
    expect(senderBalance).toBe(80);
    expect(recipientBalance).toBe(8);
    alloRes = await getAllowance(
      assetID,
      accounts[1].address,
      accounts[2].address
    );
    allowance = JSON.parse(alloRes.succeedData).value;
    expect(allowance).toBe(0);
  });
});
