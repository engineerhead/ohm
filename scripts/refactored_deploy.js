import { LCDClient, MnemonicKey, MsgExecuteContract } from '@terra-money/terra.js';


import { storeCode, instantiateContract, sendTx, queueAndToggle } from './helpers.js';
// import { send } from 'process';

// test1 key from localterra accounts
const mk = new MnemonicKey({
//   mnemonic: 'satisfy adjust timber high purchase tuition stool faith fine install that you unaware feed domain license impose boss human eager hat rent enjoy dawn'
    mnemonic: 'spatial forest elevator battle also spoon fun skirt flight initial nasty transfer glory palm drama gossip remove fan joke shove label dune debate quick'
})

// connect to localterra
const terra = new LCDClient({
    URL: 'https://bombay-lcd.terra.dev',
    chainID: 'bombay-12',
});

//connect to localterra
// const terra = new LCDClient({
//     URL: "http://localhost:1317",
//     chainID: "localterra",
// });


const wallet = terra.wallet(mk);

const zero_address = "terra000000000000000000000000000000000";
let treasuryAddress = "terra1ac7t9ug0lanc5nun9auy9a2qa7z6nvjgsnquq9";
let bondAddress = "terra1zhagpufp0w4n93yfwjv0png4mtsdaw9lnulz35";
let slumAddress = "terra1wshph3059fe5exz68w3shmr449f7n23n6y3u0g";
let stakingAddress = "terra1wjz4hmnrdm4ycn8mnrcjq84j4z9cxe5mmysh53";
let distributorAddress = "";

console.log("Big Bang");



/* TREASURY INSTANTIATION */
let treasuryCodeId = await storeCode(wallet, terra, "../artifacts/lumen_treasury.wasm");
const treasuryInstantiateMsg = {
    admin:  wallet.key.accAddress,
    sLUM: wallet.key.accAddress,
    blocks_needed_for_queue: 0
};
treasuryAddress = await instantiateContract(wallet , terra, treasuryCodeId, treasuryInstantiateMsg);
console.log("TREASURY_ADDRESS: " + '"' + treasuryAddress + '"');

// /* SLUMEN INSTANTIATION */
let slumCodeId = await storeCode(wallet, terra, "../artifacts/s_lumen_cw20.wasm");
const slumInstantiateMsg = {
    name: "Staked Lumen",
    symbol: "sLUM",
    decimals: 9,
    admin: wallet.key.accAddress
};
slumAddress = await instantiateContract(wallet , terra, slumCodeId, slumInstantiateMsg);
console.log("SLUM Instantiated");

/**
 * Distributor Instantiation
 */
let distributorCodeId = await storeCode(wallet, terra, "../artifacts/lumen_distributor.wasm");
const distributorInstantiateMsg = {
    lum: treasuryAddress,
    epoch_length: 28800,
    next_epoch: Date.now()
}
distributorAddress = await instantiateContract(wallet, terra, distributorCodeId, distributorInstantiateMsg);

/**
 * STAKING INSTANTIATION
 */
let stakingCodeId = await storeCode(wallet, terra, "../artifacts/lumen_staking.wasm");
const stakingInstantiateMsg = {
    bonds: [],
    admin: wallet.key.accAddress,
    lum: treasuryAddress,
    slum: slumAddress,
    distributor: distributorAddress,
    epoch_number: 1,
    epoch_block: Date.now()
};
stakingAddress = await instantiateContract(wallet , terra, stakingCodeId, stakingInstantiateMsg);
console.log("Staking Instantiated");


/**
 * INIT SLUM
 */
const initSlum = new MsgExecuteContract(wallet.key.accAddress, slumAddress, {
    "initialize": {
        staking_contract_addr: stakingAddress
    }
  });
await sendTx(wallet, terra, initSlum);


/* UST BOND INSTANTIATION */
let  bondCodeId = await storeCode(wallet, terra, "../artifacts/lumen_bond_depository.wasm");
const bondInstantiateMsg = {
    admin: wallet.key.accAddress,
    treasury: treasuryAddress,
        dao: "terra17lmam6zguazs5q5u6z5mmx76uj63gldnse2pdp",
        staking: stakingAddress,
        // staking_helper: treasuryAddress,
        // bond_calculator: zero_address,
        // use_helper: false,
        // is_liquidity_bond:false,
        total_debt: 0,
        last_decay: 0,
};
bondAddress = await instantiateContract(wallet , terra, bondCodeId, bondInstantiateMsg);
console.log("Bond Instantiated");


/** INITIALIZE BOND */
const initBond = new MsgExecuteContract(wallet.key.accAddress, bondAddress, {
    "init": {
        control_variable: 369,
        vesting_term: 28800,
        minimum_price: 50000,
        max_payout: 50,
        fee: 1000,
        max_debt: 1000000000000000,
        initial_debt: 0
    }
  });
  await sendTx(wallet, terra, initBond);
console.log("Bond Initialized")

const setStaking = new MsgExecuteContract(wallet.key.accAddress, bondAddress, {
    "set_staking": {
        staking: stakingAddress,
        helper: stakingAddress
    }
  });
  await sendTx(wallet, terra, setStaking);
console.log("Set staking in Bond")


/*  QUEUE & TOGGLE DEPLOYER in TREASURY as RESERVE/LIQUIDITY DEPOSITOR */
await queueAndToggle(
    wallet, 
    terra, 
    wallet.key.accAddress, 
    1, 
    treasuryAddress, 
    wallet.key.accAddress
);
console.log("QUEUE AND TOGGLE DEPLOYER AS RESERVE SPENDER");
await queueAndToggle(
        wallet, 
        terra, 
        wallet.key.accAddress, 
        0, 
        treasuryAddress, 
        wallet.key.accAddress
    );
await queueAndToggle(
    wallet, 
    terra, 
    wallet.key.accAddress, 
    4, 
    treasuryAddress, 
    wallet.key.accAddress
);
console.log("QUEUE & TOGGLE DEPLOYER in TREASURY as RESERVE/LIQUIDITY DEPOSITOR");

/*  QUEUE & TOGGLE UST BOND in TREASURY as RESERVE/LIQUIDITY DEPOSITOR */
await queueAndToggle(
        wallet, 
        terra, 
        wallet.key.accAddress, 
        0, 
        treasuryAddress, 
        bondAddress
    );
await queueAndToggle(
    wallet, 
    terra, 
    wallet.key.accAddress, 
    4, 
    treasuryAddress, 
    bondAddress
);
console.log("QUEUE & TOGGLE UST BOND in TREASURY as RESERVE/LIQUIDITY DEPOSITOR");

await queueAndToggle(
    wallet, 
    terra, 
    wallet.key.accAddress, 
    8, 
    treasuryAddress, 
    distributorAddress
);
console.log("QUEUE AND TOGGLE DISTRIBUTOR AS REWARDS MANAGER");
const addStakingToDistributor = new MsgExecuteContract(wallet.key.accAddress, distributorAddress, {
    "add_recipient": {
        recipient: stakingAddress,
        reward_rate: 3000
    }
  });
  await sendTx(wallet, terra, addStakingToDistributor);
console.log("Added staing as recipient to distributor");

const depositTreasury = new MsgExecuteContract(wallet.key.accAddress, treasuryAddress, {
    "deposit": {
        amount: 9000000000000,
        profit: 8400,
        depositor: wallet.key.accAddress 
    }
  });
await sendTx(wallet, terra, depositTreasury);
// const transferTreasury = new MsgExecuteContract(wallet.key.accAddress, treasuryAddress, {
//     "transfer": {
//         amount: "9000",
//         recipient: "terra17lmam6zguazs5q5u6z5mmx76uj63gldnse2pdp"
//     }
//   });
// let trans =  await sendTx(wallet, terra, transferTreasury);
// console.log(trans.logs[0].eventsByType);

console.log("SLUM_ADDRESS: " + '"' + slumAddress + '"' + ',');
console.log("STAKING_ADDRESS: " + '"' + stakingAddress + '"' + ',');
console.log("TREASURY_ADDRESS: " + '"' + treasuryAddress + '"' + ',');
console.log("BOND_ADDRESS: " + '"' + bondAddress + '"' + ',')
console.log("DISTRIBUTOR_ADDRESS: " + '"' + distributorAddress + '"' + ',')

// let result = await terra.wasm.contractQuery(
//     treasuryAddress,
//     {"balance": { 
//         "address": wallet.key.accAddress
//     } } // query msg
//   );

//   console.log(result)

//   let result2 = await terra.wasm.contractQuery(
//     bondAddress,
//     {"bond_info": { 
//         "address": wallet.key.accAddress
//     } } // query msg
//   );

//   console.log(result2)

// const redeemBond = new MsgExecuteContract(wallet.key.accAddress, bondAddress, {
//     "redeem": {
//         stake: false, 
//         recipient: wallet.key.accAddress
//     }
//   },
// //   { uusd: 100000000 }
//   );
//   console.log(await sendTx(wallet, terra, redeemBond));

//   let result3 = await terra.wasm.contractQuery(
//     bondAddress,
//     {"bond_info": { 
//         "address": wallet.key.accAddress
//     } } // query msg
//   );

//   console.log(result3)

//   result = await terra.wasm.contractQuery(
//     treasuryAddress,
//     {"balance": { 
//         "address": wallet.key.accAddress
//     } } // query msg
//   );

//   console.log(result)

// const depositBond = new MsgExecuteContract(wallet.key.accAddress, bondAddress, {
//     "deposit": {
//         // amount: 1000000000000000, 
//         max_price: 60000,
//         // depositor: wallet.key.accAddress
//     }
//   },
//   { uusd: 100000000 }
//   );
// let result = await sendTx(wallet, terra, depositBond);
// console.log(result.logs[0].events[0].attributes);
// console.log(result.logs[0].events[1].attributes);
// console.log(result.logs[0].events[2].attributes);
// console.log(result.logs[0].events[3].attributes);

