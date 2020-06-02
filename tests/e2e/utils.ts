import fetch from "node-fetch";
import { createHttpLink } from "apollo-link-http";
import { InMemoryCache } from "apollo-cache-inmemory";
import ApolloClient from "apollo-client";
import { readFileSync } from "fs";
import { Muta } from "muta-sdk";
import { parse as toml_parse } from "toml";

export const CHAIN_CONFIG = toml_parse(readFileSync("./chain.toml", "utf-8"));
export const GENESIS = toml_parse(readFileSync("./genesis.toml", "utf-8"));

export const CHAIN_ID =
  "0xb6a4d7da21443f5e816e8700eea87610e6d769657d6b8ec73028457bf2ca4036";
export const API_URL = process.env.API_URL || "http://localhost:8000/graphql";
export const client = new ApolloClient({
  link: createHttpLink({
    uri: API_URL,
    fetch: fetch
  }),
  cache: new InMemoryCache(),
  defaultOptions: { query: { fetchPolicy: "no-cache" } }
});
export const muta = new Muta({
  endpoint: API_URL,
  chainId: CHAIN_ID
});
export const mutaClient = muta.client();

export function makeid(length: number) {
  var result = "";
  var characters = "abcdef0123456789";
  var charactersLength = characters.length;
  for (var i = 0; i < length; i++) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
  }
  return result;
}

export function getNonce() {
  return makeid(64);
}

export function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

import * as _ from "lodash";
const mnemonic = Muta.hdWallet.generateMnemonic();
export const wallet = new Muta.hdWallet(mnemonic);
export const accounts = _.range(20).map(i => wallet.deriveAccount(i));
export const admin = Muta.accountFromPrivateKey(
  "0x2b672bb959fa7a852d7259b129b65aee9c83b39f427d6f7bded1f58c4c9310c2"
);

const asset_genesis = JSON.parse(
  _.find(GENESIS.services, o => o.name === "asset").payload
);
// console.log(asset_genesis);
export const fee_asset_id = asset_genesis.id;
export const fee_account = asset_genesis.fee_account;

export function str2hex(s) {
  return Buffer.from(s, "utf8").toString("hex");
}
