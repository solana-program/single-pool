import { getAddressCodec, getBase64Encoder } from '@solana/kit';

import { PoolAddress, VoteAccountAddress } from './addresses.js';

export * from './addresses.js';
export * from './instructions.js';
export * from './transactions.js';

export async function getVoteAccountAddressForPool(
  rpc: any, // XXX not exported: Rpc<GetAccountInfoApi>,
  poolAddress: PoolAddress,
  abortSignal?: AbortSignal,
): Promise<VoteAccountAddress> {
  const { value: poolAccount } = await rpc
    .getAccountInfo(poolAddress, { encoding: 'base64' })
    .send({ abortSignal });
  const poolData = poolAccount && getBase64Encoder().encode(poolAccount.data[0]);
  if (!(poolData && poolData[0] === 1)) {
    throw 'invalid pool address';
  }
  return getAddressCodec().decode(poolData.slice(1)) as VoteAccountAddress;
}
