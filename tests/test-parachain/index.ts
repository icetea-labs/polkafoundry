import { ApiPromise } from '@polkadot/api';

// initialise via static create
var test = async function () {
  let api = await ApiPromise.create();

  const ADDR = '5DTestUPts3kjeXSTMyerHihn1uwMfLj8vU8sqF7qYrFabHE';
  // Do something
  console.log(api.genesisHash.toHex());
}

test();