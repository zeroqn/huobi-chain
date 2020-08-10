import { parse } from 'toml';
import { find } from 'lodash';
import { readFileSync } from 'fs';
import { utils } from '@mutadev/muta-sdk';
import { Client } from '@mutadev/client';
import { Account } from '@mutadev/account';
import { BigNumber } from '@mutadev/shared';
import { AssetService, InterpreterType, RISCVService } from 'huobi-chain-sdk';


const { hexToNum } = utils;

/*
These 4 keypairs are for test
0x0000000000000000000000000000000000000000000000000000000000000001
0x0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
hb10e0525sfrf53yh2aljmm3sn9jq5njk7lsekwy5

0x0000000000000000000000000000000000000000000000000000000000000002
0x02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5
hb19dddt3retspx298cx9785g27yxxue4k0df2c2y

0x0000000000000000000000000000000000000000000000000000000000000003
0x02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9
hb1dqf7hymzxuhw7csq7wcahslcr9n3ewnfd8ecu6

0x0000000000000000000000000000000000000000000000000000000000000004
0x02e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13
hb1rml500p6zzj96jerpdw3pcmh28lx4fccw9ljut
 */


const admin: Account = Account.fromPrivateKey('0x0000000000000000000000000000000000000000000000000000000000000001');

const player2: Account = Account.fromPrivateKey('0x0000000000000000000000000000000000000000000000000000000000000002');
const player3: Account = Account.fromPrivateKey('0x0000000000000000000000000000000000000000000000000000000000000003');
const player4: Account = Account.fromPrivateKey('0x0000000000000000000000000000000000000000000000000000000000000004');

const adminClient = new Client({
  defaultCyclesLimit: '0xffffffff',
  maxTimeout:10000,
  account: admin,
});

const player2Client = new Client({
  defaultCyclesLimit: '0xffffffff',
  maxTimeout:10000,
  account: player2,
});

const player3Client = new Client({
  defaultCyclesLimit: '0xffffffff',
  maxTimeout:10000,
  account: player3,
});

const randomString = require('randomstring');
const genesis = parse(readFileSync('config/genesis.toml', 'utf-8'));

const nativeAssetId = JSON.parse(
    find(genesis.services, (s) => s.name === 'asset').payload,
).id;

const governance = JSON.parse(
  find(genesis.services, (s) => s.name === 'governance').payload,
);

export function genRandomString(prefix: String = 'r', length: number = 12) {
  expect(prefix.length <= length);
  return prefix + randomString.generate(length - prefix.length);
}

export function genRandomStrings(size: number = 3, prefix: String = 't', length: number = 12) {
  const names = new Array(0);

  for (let i = 0; i < size; i++) {
    names.push(genRandomString(prefix, length));
  }

  return names;
}

export function genRandomAccount() {
  const randomPriKey = randomString.generate({
    charset: '0123456789abcdef',
    length: 64,
  });
  return Account.fromPrivateKey(`0x${randomPriKey}`);
}

export function genRandomHex(length = 4){
  const randomPriKey = randomString.generate({
    charset: '0123456789abcdef',
    length,
  });
  return '0x'+ randomPriKey;
}

export function genRandomInt(min = 0x0, max = 0xfffffffff) {
  min = Math.ceil(min);
  max = Math.floor(max);
  return Math.floor(Math.random() * (max - min)) + min;
}

export async function transfer(to: string, value: number) {
  const service = new AssetService(adminClient, admin);
  await service.write.transfer({
    asset_id: nativeAssetId,
    to,
    value,
    memo: 'transfer',
  });
}

export async function get_balance(user: string) {
  const service = new AssetService(adminClient, admin);
  const res0 = await service.read.get_balance({
    asset_id: nativeAssetId,
    user,
  });
  return new BigNumber(res0.succeedData.balance);
}

export async function deploy(code: string, initArgs: string) {
  const service = new RISCVService(adminClient, admin);
  const res0 = await service.write.grant_deploy_auth({
    addresses: [admin.address],
  });
  expect(Number(res0.response.response.code)).toBe(0);

  const res1 = await service.write.deploy({
    code,
    intp_type: InterpreterType.Binary,
    init_args: initArgs,
  });
  expect(Number(res1.response.response.code)).toBe(0);

  const contractAddress = res1.response.response.succeedData.address;
  const res2 = await service.write.approve_contracts({
    addresses: [contractAddress],
  });
  expect(Number(res2.response.response.code)).toBe(0);
  return contractAddress;
}

export {
  admin, adminClient, player2,player2Client,player3,player3Client,player4,governance, hexToNum, nativeAssetId,
};
