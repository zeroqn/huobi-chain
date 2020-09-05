import { Address } from '@mutadev/types';
import {AssetService, KycService} from 'huobi-chain-sdk';
import {
  admin, adminClient, player2 as orgAdmin, player2Client as orgAdminClient,
  genRandomString, genRandomStrings, genRandomAccount, transfer, nativeAssetId, player2
} from './common/utils';
import {Client} from "@mutadev/client";

const kycServiceAdmin = new KycService(adminClient, admin);
const kycServiceOrgAdmin = new KycService(orgAdminClient, orgAdmin);

async function register_org(service = kycServiceAdmin, expectCode = 0, nameLen = 12, tagNum = 3, tagLen = 12) {
  const orgName = genRandomString('a', nameLen);

  // pre-check
  const res0 = await service.read.get_org_info(orgName);
  expect(Number(res0.code)).toBe(0x67);

  const res01 = await service.read.get_orgs();
  expect(Number(res01.code)).toBe(0);
  expect(res01.succeedData.indexOf(orgName)).toBe(-1);

  const res02 = await service.read.get_org_supported_tags(orgName);
  expect(Number(res02.code)).toBe(0x67);

  // register org
  const description = genRandomString('d', 50);
  const supportedTags = genRandomStrings(tagNum, 'r', tagLen);
  const res1 = await service.write.register_org({
    name: orgName,
    description,
    admin: orgAdmin.address,
    supported_tags: supportedTags,
  });
  const code = Number(res1.response.response.code);
  expect(code).toBe(expectCode);

  // post-check
  if (code == 0) {
    const res2 = await service.read.get_org_info(orgName);
    const data2 :any = res2.succeedData;
    expect(Number(res2.code)).toBe(0);
    expect(data2.name).toBe(orgName);
    expect(data2.description).toBe(description);
    expect(data2.admin).toBe(orgAdmin.address);
    expect(JSON.stringify(data2.supported_tags)).toBe(JSON.stringify(supportedTags));
    expect(data2.approved).toBe(false);

    const res3 = await service.read.get_orgs();
    expect(Number(res3.code)).toBe(0);
    expect(res3.succeedData.indexOf(orgName)).not.toBe(-1);

    const res4 = await service.read.get_org_supported_tags(orgName);
    expect(Number(res4.code)).toBe(0);
    expect(JSON.stringify(res4.succeedData)).toBe(JSON.stringify(supportedTags));
  }

  return { org_name: orgName, tags: supportedTags };
}

async function approve(orgName: string, approved = true, service = kycServiceAdmin, expectCode = 0) {
  const res0 = await service.write.change_org_approved({
    org_name: orgName,
    approved,
  });

  const code = Number(res0.response.response.code);
  expect(code).toBe(expectCode);

  if (code == 0) {
    const res1 :any= await service.read.get_org_info(orgName);
    expect(res1.succeedData.approved).toBe(approved);
  }
}

async function update_supported_tags(orgName: string, service = kycServiceOrgAdmin, expectCode = 0, tagNum = 3, tagLen = 12) {
  const newSupportedTags = genRandomStrings(tagNum, 'r', tagLen);
  const res0 = await service.write.update_supported_tags({
    org_name: orgName,
    supported_tags: newSupportedTags,
  });
  const code = Number(res0.response.response.code);
  expect(code).toBe(expectCode);

  if (code == 0) {
    const res2 :any= await service.read.get_org_info(orgName);
    expect(JSON.stringify(res2.succeedData.supported_tags)).toBe(JSON.stringify(newSupportedTags));
  }

  return newSupportedTags;
}

async function update_user_tags(orgName: string, supportedTags: Array<string>, service = kycServiceOrgAdmin, expectCode = 0, valNum = 3, valLen = 12) {
  const user = genRandomAccount().address;

  const tags = <Record<string, Array<string>>>{};
  // set value of each tag
  // value should be an array of string
  supportedTags.map((tag) => {
    tags[tag] = genRandomStrings(valNum, 'm', valLen);
  });

  const res0 = await service.write.update_user_tags({
    org_name: orgName,
    user,
    tags,
  });
  const code = Number(res0.response.response.code);
  expect(code).toBe(expectCode);

  if (code == 0) {
    const res1 = await service.read.get_user_tags({
      org_name: orgName,
      user,
    });
    expect(Number(res1.code)).toBe(0);
    expect(res1.succeedData.length).toBe(tags.length);
    for (const k in res1.succeedData) {
      expect(JSON.stringify(res1.succeedData[k])).toBe(JSON.stringify(tags[k]));
    }
  }

  return { user, values: tags };
}

async function change_service_admin(newAdmin: Address, service = kycServiceAdmin, expectCode = 0) {
  const res0 = await service.write.change_service_admin({
    new_admin: newAdmin,
  });
  const code = Number(res0.response.response.code);
  expect(code).toBe(expectCode);
}

async function change_org_admin(orgName: string, newAdmin: Address, service = kycServiceOrgAdmin, expectCode = 0) {
  const res0 = await service.write.change_org_admin({
    name: orgName,
    new_admin: newAdmin,
  });
  const code = Number(res0.response.response.code);
  expect(code).toBe(expectCode);
}

async function eval_user_tag_expression(user: Address, expression: string, expectCode = 0, result = true) {
  const res = await kycServiceAdmin.read.eval_user_tag_expression({
    user,
    expression,
  });
  const code = Number(res.code);
  expect(code).toBe(expectCode);
  if (code == 0) {
    expect(res.succeedData).toBe(result);
  }
}

describe('kyc service API test via huobi-sdk-js', () => {

  beforeAll(async () =>{
    const assetServiceAdmin = new AssetService(adminClient, admin);

    const res = await assetServiceAdmin.write.transfer({
      asset_id: nativeAssetId,
      to:orgAdmin.address,
      value: 0xfff26635,
      memo: 'transfer',
    });
    const code = Number(res.response.response.code);
    expect(code).toBe(0);
  })

  test('register_org', async () => {
    await register_org();
  });

  test('change_org_approved', async () => {
    // register org
    const res = await register_org();
    const orgName = res.org_name;
    // approve
    await approve(orgName, true);
    // disapprove
    await approve(orgName, false);
  });

  test('update_supported_tags', async () => {
    // register org
    const res = await register_org();
    const orgName = res.org_name;
    // update supported tags
    await update_supported_tags(orgName);
  });

  test('update_user_tags', async () => {
    // register org
    const res = await register_org();
    const orgName = res.org_name;
    const { tags } = res;
    // update user tags before approved
    await update_user_tags(orgName, tags, kycServiceOrgAdmin, 0x6c);
    // approve
    await approve(orgName, true);
    // update user tags after approved
    await update_user_tags(orgName, tags);
  });

  test('update_user_tags_fails', async () => {
    // register org
    const res = await register_org();
    const orgName = res.org_name;
    const notSupportedTags = genRandomStrings(3, 'x', 12);

    // approve
    await approve(orgName, true);
    // update user tags after approved
    await update_user_tags(orgName, notSupportedTags,kycServiceOrgAdmin,110);
  });

  test('change_service_admin', async () => {

    const randomAccount = genRandomAccount();
    await transfer(randomAccount.address, 999999999);

    const randomAccountClient = new Client({
      defaultCyclesLimit: '0xffffffff',
      maxTimeout:10000,
      account: randomAccount,
    });

    // change_service_admin
    await change_service_admin(randomAccount.address,);

    const info = await kycServiceOrgAdmin.read.get_admin();

    expect(info.succeedData).toBe(randomAccount.address);

    const kycServiceRandom = new KycService(randomAccountClient, randomAccount);

    await change_service_admin(admin.address,kycServiceRandom);

    const info2 = await kycServiceRandom.read.get_admin();

    expect(info2.succeedData).toBe(admin.address);

    //must change back
  });

  test('change_org_admin', async () => {
    // register org and approve
    const res = await register_org();
    const orgName = res.org_name;
    const { tags } = res;
    await approve(orgName);

    // create new account and transfer coins
    const randomAccount = genRandomAccount();
    await transfer(randomAccount.address, 999999999);

    // before update check update_user_tags, change_org_admin
    await change_org_admin(orgName, randomAccount.address, kycServiceOrgAdmin, 0x0);

    const info = await kycServiceOrgAdmin.read.get_org_info(orgName);
    // @ts-ignore
    expect(info.succeedData.admin).toBe(randomAccount.address);

  });

  // test eval_user_tag_expression
  test('eval_user_tag_expression', async () => {
    // register org and approve
    const res = await register_org();
    const orgName = res.org_name;
    const supportedTags = res.tags;
    await approve(orgName);
    // update user tags after approved
    const res1 = await update_user_tags(orgName, supportedTags);
    const { user } = res1;
    const { values } = res1;
    // test basic expression
    const expression_0 = `${orgName}.${supportedTags[0]}@\`${values[supportedTags[0]][0]}\``;
    await eval_user_tag_expression(user, expression_0, 0, true);

    const randomAddress = genRandomAccount().address;
    await eval_user_tag_expression(randomAddress, expression_0, 0, false);

    const expression_1 = `${orgName}.${supportedTags[0]}@\`${values[supportedTags[1]][0]}\``;
    await eval_user_tag_expression(user, expression_1, 0, false);

    // test complex expression
    const expression_2 = `(${orgName}.${supportedTags[0]}@\`${values[supportedTags[0]][0]
    }\` || ${orgName}.${supportedTags[1]}@\`${values[supportedTags[0]][0]}\`) && ${
      orgName}.${supportedTags[2]}@\`${values[supportedTags[2]][2]}\``;
    await eval_user_tag_expression(user, expression_2, 0, true);

  });
});
