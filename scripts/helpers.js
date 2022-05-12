import { MsgStoreCode, isTxError, MsgInstantiateContract, MsgExecuteContract } from '@terra-money/terra.js';
import * as fs from 'fs';

export async function instantiateContract(wallet, terra, storeCodeId, msg){
    const contractMsg = new MsgInstantiateContract(
        wallet.key.accAddress,
        wallet.key.accAddress,
        storeCodeId, // code ID
        msg,
        {}
      );
    const result = await sendTx(wallet, terra, contractMsg);
    return result.logs[0].eventsByType.instantiate_contract.contract_address[0];
}

export async function storeCode(wallet, terra, path){
    const file = fs.readFileSync(path).toString('base64');
    const storeCode = new MsgStoreCode(wallet.key.accAddress, file);
    const tx = await sendTx(wallet, terra, storeCode);
    return parseInt(tx.logs[0].eventsByType.store_code.code_id[0]);
}

export async function sendTx(wallet, terra, msg){

    const txToSend = await wallet.createAndSignTx({
        msgs: [msg],
    });
    const txResult = await terra.tx.broadcast(txToSend);

    if (isTxError(txResult)) {
        throw new Error(
          `instantiate failed. code: ${txResult.code}, codespace: ${txResult.codespace}, `
        );
      }

    return txResult;
    
}

export async function queueAndToggle(wallet, terra, sender, managingKey, treasuryAddress, address){
  const queue_msg = new MsgExecuteContract(sender, treasuryAddress, {
    "queue": {
        "managing": managingKey,
        "address": address 
    }
  });
  await sendTx(wallet, terra, queue_msg);

  const toggle_msg = new MsgExecuteContract(sender, treasuryAddress, {
    "toggle": {
        "managing": managingKey,
        "address":  address 
    }
  });
  await sendTx(wallet, terra, toggle_msg);

}