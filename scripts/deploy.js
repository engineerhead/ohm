import { LCDClient, MsgStoreCode, MnemonicKey, isTxError, MsgInstantiateContract } from '@terra-money/terra.js';

import * as fs from 'fs';

// test1 key from localterra accounts
const mk = new MnemonicKey({
  mnemonic: 'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius'
})
const zero_address = "terra000000000000000000000000000000000";
// connect to localterra
const terra = new LCDClient({
  URL: 'http://localhost:1317',
  chainID: 'localterra'
});

const wallet = terra.wallet(mk);

const storeCode = new MsgStoreCode(
  wallet.key.accAddress,
  fs.readFileSync('../artifacts/lumen_treasury.wasm').toString('base64')
);
const storeCodeTx = await wallet.createAndSignTx({
  msgs: [storeCode],
});
const storeCodeTxResult = await terra.tx.broadcast(storeCodeTx);

// console.log(storeCodeTxResult);

if (isTxError(storeCodeTxResult)) {
  throw new Error(
    `store code failed. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}, raw_log: ${storeCodeTxResult.raw_log}`
  );
}

const {
  store_code: { code_id },
} = storeCodeTxResult.logs[0].eventsByType;



const instantiate = new MsgInstantiateContract(
  wallet.key.accAddress,
  wallet.key.accAddress,
  code_id[0], // code ID
  {
    admin:  wallet.key.accAddress,
    sLUM: wallet.key.accAddress,
    blocks_needed_for_queue: 0
},
  { uluna: 10000000, ukrw: 1000000 }
);

const instantiateTx = await wallet.createAndSignTx({
  msgs: [instantiate],
});
const instantiateTxResult = await terra.tx.broadcast(instantiateTx);

console.log(instantiateTxResult);

if (isTxError(instantiateTxResult)) {
  throw new Error(
    `instantiate failed. code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}, raw_log: ${instantiateTxResult.raw_log}`
  );
}

const {
  instantiate_contract: { contract_address },
} = instantiateTxResult.logs[0].eventsByType;
console.log(contract_address[0])