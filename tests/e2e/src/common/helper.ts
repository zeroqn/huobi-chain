import { retry } from '@mutadev/client';
import { Account } from '@mutadev/account';
import { Hash } from '@mutadev/types';
import {
  adminClient,
  admin,
  nativeAssetId,
} from "./utils";

export async function transfer(
  txSender: Account,
  assetID: any,
  to: any,
  value: any,
) {
  const payload = {
    asset_id: assetID,
    to,
    value,
  };

  const tx = await adminClient.composeTransaction({
    method: 'transfer',
    payload,
    serviceName: 'asset',
    sender: txSender.address,
  });

  const signedTx = txSender.signTransaction(tx);
  const hash = await adminClient.sendTransaction(signedTx);
  const receipt = await retry(() => adminClient.getReceipt(hash));

  return receipt;
}

export async function addFeeTokenToAccounts(accounts: Array<Hash>) {
  await Promise.all(
    accounts.map((account) => transfer(admin, nativeAssetId, account, 10000)),
  );
}

export async function getBalance(assetID: string, user: string) {
  const res = await adminClient.queryService({
    serviceName: 'asset',
    method: 'get_balance',
    payload: JSON.stringify({
      asset_id: assetID,
      user,
    }),
  });

  return res;
}
