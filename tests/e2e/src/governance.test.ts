/* eslint-env node, jest */
import { readFileSync } from 'fs';
import { GovernanceService,MetadataService} from 'huobi-chain-sdk';
import {
  admin, adminClient, deploy, genRandomAccount, genRandomInt, get_balance, governance, transfer,
    player2,player2Client
} from './common/utils';

const governanceService = new GovernanceService(adminClient, admin);
const metadataService = new MetadataService(adminClient, admin);

const txFailureFee = governance.info.tx_failure_fee;
const txFloorFee = governance.info.tx_floor_fee;
const profitDeductRate = governance.info.profit_deduct_rate_per_million;
const txFeeDiscount: Array<any> = governance.info.tx_fee_discount;

describe('governance service', () => {
  test('test_update_metadata', async () => {

    const res = await governanceService.write.update_metadata( {
      "timeout_gap": 1234,
      "cycles_limit": 999999999999,
      "cycles_price": 1,
      "interval": 1500,
      "verifier_list": [
        {
          "bls_pub_key": "0x04102947214862a503c73904deb5818298a186d68c7907bb609583192a7de6331493835e5b8281f4d9ee705537c0e765580e06f86ddce5867812fceb42eecefd209f0eddd0389d6b7b0100f00fb119ef9ab23826c6ea09aadcc76fa6cea6a32724",
          "pub_key": "0x02ef0cb0d7bc6c18b4bea1f5908d9106522b35ab3c399369605d4242525bda7e60",
          "address": "hb14e0lmgck835vm2dfm0w3ckv6svmez8fd8lvdrp",
          "propose_weight": 1,
          "vote_weight": 1
        }
      ],
      "propose_ratio": 30,
      "prevote_ratio": 20,
      "precommit_ratio": 20,
      "brake_ratio": 14,
      "tx_num_limit": 9000,
      "max_tx_size": 10485760
    })
    expect(Number(res.response.response.code)).toBe(0);

    const res0 = await metadataService.read.get_metadata();
    expect(Number(res0.code)).toBe(0);
    expect(res0.succeedData.interval).toBe(1500);
    expect(res0.succeedData.propose_ratio).toBe(30);
    expect(res0.succeedData.prevote_ratio).toBe(20);
    expect(res0.succeedData.precommit_ratio).toBe(20);
    expect(res0.succeedData.brake_ratio).toBe(14);

    // set it back
    const res1 = await governanceService.write.update_metadata( {
      "timeout_gap": 99999,
      "cycles_limit": 999999999999,
      "cycles_price": 1,
      "interval": 3000,
      "verifier_list": [
        {
          "bls_pub_key": "0x04102947214862a503c73904deb5818298a186d68c7907bb609583192a7de6331493835e5b8281f4d9ee705537c0e765580e06f86ddce5867812fceb42eecefd209f0eddd0389d6b7b0100f00fb119ef9ab23826c6ea09aadcc76fa6cea6a32724",
          "pub_key": "0x02ef0cb0d7bc6c18b4bea1f5908d9106522b35ab3c399369605d4242525bda7e60",
          "address": "hb14e0lmgck835vm2dfm0w3ckv6svmez8fd8lvdrp",
          "propose_weight": 1,
          "vote_weight": 1
        }
      ],
      "propose_ratio": 15,
      "prevote_ratio": 10,
      "precommit_ratio": 10,
      "brake_ratio": 7,
      "tx_num_limit": 9000,
      "max_tx_size": 10485760
    })

    expect(Number(res1.response.response.code)).toBe(0);

  });

  test('test_update_interval', async () => {

    const res = await governanceService.write.update_interval( {
      "interval": 3000,
    });

    expect(Number(res.response.response.code)).toBe(0);
  });

  test('test_set_govern_info', async () => {

    const res = await governanceService.write.set_govern_info( {
      "inner": {
        "admin": "hb10e0525sfrf53yh2aljmm3sn9jq5njk7lsekwy5",
        "tx_failure_fee": 100,
        "tx_floor_fee": 10,
        "profit_deduct_rate_per_million": 10,
        "tx_fee_discount": [
          {"threshold": 10, "discount_percent": 10}
        ],
        "miner_benefit": 3
      }
    })

    expect(Number(res.response.response.code)).toBe(0);

    const info = (await governanceService.read.get_govern_info()).succeedData;
    expect(info.tx_failure_fee).toBe(100);
    expect(info.tx_floor_fee).toBe(10);

    // set it back
    const res1 = await governanceService.write.set_govern_info( {
      "inner": {
        "admin": "hb10e0525sfrf53yh2aljmm3sn9jq5njk7lsekwy5",
        "tx_failure_fee": 1000,
        "tx_floor_fee": 100,
        "profit_deduct_rate_per_million": 10,
        "tx_fee_discount": [
          {"threshold": 10, "discount_percent": 10}
        ],
        "miner_benefit": 3
      }
    })
    expect(Number(res1.response.response.code)).toBe(0);

  });
});
