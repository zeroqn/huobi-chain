/* eslint-env node, jest */
import { retry } from '@mutadev/client';
import {
  adminClient,
  hexToNum,
  admin,
  // eslint-disable-next-line
} from "./common/utils";

describe('basic API test via muta-sdk-js', () => {
  test('getLatestBlockHeight', async () => {
    const currentHeight = await adminClient.getLatestBlockHeight();
    expect(currentHeight).toBeGreaterThan(0);
  });

  test('getBlock', async () => {
    const block = await adminClient.getBlock('0x01');
    expect(block).not.toBe(undefined);
    if (block != undefined) {
      expect(hexToNum(block.header.height)).toBe(1);
    }
  });

  test('send_tx_exceed_cycles_limit', async () => {
    const tx = await adminClient.composeTransaction({
      method: 'create_asset',
      payload: {
        name: 'Muta Token',
        symbol: 'MT',
        supply: 1000000000,
      },
      serviceName: 'asset',
      sender: admin.address,
    });
    tx.cyclesLimit = '0xE8D4A51FFF';
    const signedTx = admin.signTransaction(tx);

    try {
      await adminClient.sendTransaction(signedTx);
      expect(true).toBe(false);
    } catch (err) {
      expect(err.response.errors[0].message.includes('ExceedCyclesLimit')).toBe(
        true,
      );
    }
  });

  test('send_tx_exceed_tx_size_limit', async () => {
    const tx = await adminClient.composeTransaction({
      method: 'create_asset',
      payload: {
        name: 'Muta Token',
        symbol: 'MT',
        supply: 1000000000,
        bigdata: 'a'.repeat(300000),
      },
      serviceName: 'asset',
      sender: admin.address,
    });
    const signedTx = admin.signTransaction(tx);

    try {
      await adminClient.sendTransaction(signedTx);
    } catch (err) {
      const errMsg = err.response.errors[0].message;
      expect(errMsg.includes('ExceedSizeLimit')).toBe(true);
    }
  });

  test('send tx, get tx and receipt', async () => {
    const tx = await adminClient.composeTransaction({
      method: 'create_asset',
      payload: {
        name: 'Muta Token',
        symbol: 'MT',
        supply: 1000000000,
      },
      serviceName: 'asset',
      sender: admin.address,
    });
    const signedTx = admin.signTransaction(tx);

    const hash = await adminClient.sendTransaction(signedTx);
    const receipt = await retry(() => adminClient.getReceipt(hash));
    expect(receipt.txHash).toBe(hash);

    const committedTx = await adminClient.getTransaction(hash);
    expect(committedTx).not.toBe(undefined);
    if (committedTx != undefined) {
      expect(committedTx.txHash).toBe(hash);
    }
  });
});
