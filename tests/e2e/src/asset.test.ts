/* eslint-env node, jest */
import { BigNumber } from '@mutadev/shared';
import { Address } from '@mutadev/types';
import {AssetService } from 'huobi-chain-sdk';
import {
  admin, adminClient, genRandomString, genRandomAccount, genRandomHex,nativeAssetId,
} from './common/utils';

const assetServiceAdmin = new AssetService(adminClient, admin);

async function create_asset(service = assetServiceAdmin, expectCode = 0, relayable = true,
                            nameLen = 20, symbolLen = 8, supply = 0xfffffffffff,
                            precision = 18) {
  const name = genRandomString('c', nameLen);
  const symbol = genRandomString('S', symbolLen);
  const res0 = await service.write.create_asset({
    name,
    symbol,
    admin: admin.address,
    supply,
    init_mints :[{
    addr: admin.address,
        balance: supply,
  }],
    precision,
    relayable,
  });
  const code = Number(res0.response.response.code);
  expect(Number(res0.response.response.code)).toBe(expectCode);

  if (code == 0) {
    const asset = res0.response.response.succeedData;
    expect(asset.name).toBe(name);
    expect(asset.symbol).toBe(symbol);
    expect(asset.supply).toBe(supply);
    expect(asset.precision).toBe(precision);
    expect(asset.relayable).toBe(relayable);

    const asset_id = asset.id;
    const res1 = await service.read.get_asset({ id: asset_id });
    expect(Number(res1.code)).toBe(0);
    const data = res1.succeedData;
    expect(data.name).toBe(name);
    expect(data.symbol).toBe(symbol);
    expect(data.supply).toBe(supply);
    expect(data.precision).toBe(precision);
    expect(data.relayable).toBe(relayable);

    return asset_id;
  }
  return 'null';
}

async function get_supply(assetId: string) {
  const res = await assetServiceAdmin.read.get_asset({
    id: assetId,
  });
  return new BigNumber(res.succeedData.supply);
}

async function get_native_supply() {
  return await get_supply(nativeAssetId);
}

async function get_balance(assetId: string, user: Address) {
  const res0 = await assetServiceAdmin.read.get_balance({
    asset_id: assetId,
    user,
  });
  expect(Number(res0.code)).toBe(0);
  expect(res0.succeedData.asset_id).toBe(assetId);
  expect(res0.succeedData.user).toBe(user);
  return new BigNumber(res0.succeedData.balance);
}

async function get_native_balance(user: Address) {
  return await get_balance(nativeAssetId, user);
}

async function get_allowance(assetId: string, grantor: Address, grantee: Address) {
  const res0 = await assetServiceAdmin.read.get_allowance({
    asset_id: assetId,
    grantor,
    grantee,
  });
  expect(Number(res0.code)).toBe(0);
  return new BigNumber(res0.succeedData.value);
}

async function get_native_allowance(grantor: Address, grantee: Address) {
  return await get_allowance(nativeAssetId, grantor, grantee);
}

async function transfer(assetId: string, to: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  const res = await service.write.transfer({
    asset_id: assetId,
    to,
    value,
    memo: 'transfer',
  });
  const code = Number(res.response.response.code);
  expect(code).toBe(expectCode);
}

async function native_transfer(to: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  return await transfer(nativeAssetId, to, value, service, expectCode);
}

async function approve(assetId: string, to: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  const res = await service.write.approve({
    asset_id: assetId,
    to,
    value,
    memo: 'approve',
  });
  const code = Number(res.response.response.code);
  expect(code).toBe(expectCode);
}

async function native_approve(to: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  return await approve(nativeAssetId, to, value, service, expectCode);
}

async function transfer_from(assetId: string, sender: Address, recipient: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  const res = await service.write.transfer_from({
    asset_id: assetId,
    sender,
    recipient,
    value,
    memo: 'transfer_from',
  });
  const code = Number(res.response.response.code);
  expect(code).toBe(expectCode);
}

async function native_transfer_from(sender: Address, recipient: Address, value: number, service = assetServiceAdmin, expectCode = 0) {
  return await transfer_from(nativeAssetId, sender, recipient, value, service, expectCode);
}

async function burn(assetId: string, amount: number, service = assetServiceAdmin, expectCode = 0) {
  const res1 = await service.write.burn({
    asset_id: assetId,
    amount,
    proof: genRandomHex(),
    memo: 'burn',
  });
  const code = Number(res1.response.response.code);
  expect(code).toBe(expectCode);
}

async function native_burn(amount: number, service = assetServiceAdmin, expectCode = 0) {
  return await burn(nativeAssetId, amount, service, expectCode);
}

async function mint(assetId: string, to: Address, amount: number, service = assetServiceAdmin, expectCode = 0) {
  const payload = {
    asset_id: assetId,
    to,
    amount,
    proof: genRandomHex(),
    memo: 'mint',
  }
  const res1 = await service.write.mint(payload);
  const code = Number(res1.response.response.code);
  expect(code).toBe(expectCode);
}

async function relay(assetId: string, amount: number, service = assetServiceAdmin, expectCode = 0) {
  const res1 = await service.write.relay({
    asset_id: assetId,
    amount,
    proof: genRandomHex(),
    memo: 'burn',
  });
  const code = Number(res1.response.response.code);
  expect(code).toBe(expectCode);
}

async function native_relay(amount: number, service = assetServiceAdmin, expectCode = 0) {
  return await relay(nativeAssetId, amount, service, expectCode);
}

async function change_admin(assetId: string,addr: Address, service = assetServiceAdmin, expectCode = 0) {
  const res1 = await service.write.change_admin({
    asset_id:assetId,
    new_admin: addr,
  });
  expect(Number(res1.response.response.code)).toBe(expectCode);
}

describe('asset service API test via huobi-sdk-js', () => {
  test('create_asset', async () => {
    await create_asset();
  });

  test('transfer', async () => {
    const newAccount = genRandomAccount();
    const balance_before = await get_native_balance(newAccount.address);
    const value = 0xfffff;
    await native_transfer(newAccount.address, value);
    // check balance
    const balance_after = await get_native_balance(newAccount.address);
    expect(balance_after.minus(balance_before).eq(value)).toBe(true);
  });

  test('approve and transfer_from', async () => {
    const account1 = genRandomAccount();
    const service1 = new AssetService(adminClient, account1);
    const account2 = genRandomAccount();
    // transfer
    await native_transfer(account1.address, 0xffff1111);
    // approve
    const value0 = 0xfffff;
    await native_approve(account1.address, value0);
    // get_allowance
    const al_before = await get_native_allowance(admin.address, account1.address);
    expect(al_before.minus(value0).eq(0)).toBe(true);
    // transfer_from
    const value1 = 0x65a41;
    await native_transfer_from(admin.address, account2.address, value1, service1);
    // check balance
    const al_after = await get_native_allowance(admin.address, account1.address);
    expect(al_before.minus(al_after).eq(value1)).toBe(true);
    const balance = await get_native_balance(account2.address);
    expect(balance.eq(value1)).toBe(true);
  });

  test('mint', async () => {
    const randomAccount = genRandomAccount();

    const assetServiceRandomAccount = new AssetService(adminClient, randomAccount);
    // transfer
    const value = 0xfffffff;
    await native_transfer(randomAccount.address, value);
    // create_asset
    // new asset's admin is 'admin'
    const assetId = await create_asset(assetServiceAdmin);
    // unauthorized mint
    const amount = 0x652a1fff;
    await mint(assetId, randomAccount.address, amount, assetServiceRandomAccount, 0x6d);

    const balance_before = await get_balance(assetId, randomAccount.address);
    const supply_before = await get_supply(assetId);
    // mint
    await mint(assetId, randomAccount.address, amount, assetServiceAdmin);
    // check balance
    const balance_after = await get_balance(assetId, randomAccount.address);
    expect(balance_after.minus(balance_before).eq(amount)).toBe(true);
    const supply_after = await get_supply(assetId);
    expect(supply_after.minus(supply_before).eq(amount)).toBe(true);
  });

  test('burn', async () => {
    const newAccount = genRandomAccount();
    const newService = new AssetService(adminClient, newAccount);
    // transfer
    const value = 0xffffffff;
    await native_transfer(newAccount.address, value);
    const supply_before = await get_native_supply();
    // burn
    const amount = 0x652a1fff;
    await native_burn(amount, newService);
    const supply_after = await get_native_supply();
    expect(supply_before.minus(supply_after).eq(amount)).toBe(true);
  });

  test('relay', async () => {
    const asset_id_1 = await create_asset(assetServiceAdmin,0,false);
    // test relay of unrelayable asset
    const amount = 0x3ab12451;
    await relay(asset_id_1, amount, assetServiceAdmin, 0x6f);
    // test relay of relayable asset
    await native_relay(amount);
  });

  test('change_admin', async () => {
    const newAccount = genRandomAccount();
    const newService = new AssetService(adminClient, newAccount);
    // transfer
    await native_transfer(newAccount.address, 0xfff26635);
    // change_admin
    await change_admin(nativeAssetId,newAccount.address, newService, 0x6d);
    // change_admin
    await change_admin(nativeAssetId,newAccount.address);
    // change_admin
    await change_admin(nativeAssetId,admin.address, newService);
  });

  test('drain transfer', async () => {
    const newAccount = genRandomAccount();
    // transfer
    const value = 0xfffff;
    await native_transfer(newAccount.address, value);
    // drain transfer
    const newService = new AssetService(adminClient, newAccount);
    await native_transfer(admin.address, value, newService, 0x66);
  });
});
