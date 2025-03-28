import test from 'ava';
import { start, BanksClient, ProgramTestContext } from 'solana-bankrun';
import {
  Keypair,
  PublicKey,
  Transaction,
  Authorized,
  TransactionInstruction,
  StakeProgram,
  VoteProgram,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
  getVoteAccountAddressForPool,
  findDefaultDepositAccountAddress,
  MPL_METADATA_PROGRAM_ID,
  findPoolAddress,
  findPoolStakeAddress,
  findPoolOnRampAddress,
  findPoolMintAddress,
  SinglePoolProgram,
  findMplMetadataAddress,
} from '../src';

const voteAccount = {
  pubkey: 'KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag',
  account: {
    lamports: 1300578700922,
    data: [
      'AQAAAAs8CYpjxAGc9BKIFsvo43erJeAPq9FLBOZuVf7zcXQwDtalO9ClDHolg+JcQCSa0sIFkdUQpQh5ufXK07iakuhkHwAAAAAAAAACxGIMAAAAAB8AAAADxGIMAAAAAB4AAAAExGIMAAAAAB0AAAAFxGIMAAAAABwAAAAGxGIMAAAAABsAAAAHxGIMAAAAABoAAAAIxGIMAAAAABkAAAAJxGIMAAAAABgAAAAKxGIMAAAAABcAAAALxGIMAAAAABYAAAAMxGIMAAAAABUAAAANxGIMAAAAABQAAAAOxGIMAAAAABMAAAAPxGIMAAAAABIAAAAQxGIMAAAAABEAAAARxGIMAAAAABAAAAASxGIMAAAAAA8AAAATxGIMAAAAAA4AAAAUxGIMAAAAAA0AAAAVxGIMAAAAAAwAAAAWxGIMAAAAAAsAAAAXxGIMAAAAAAoAAAAYxGIMAAAAAAkAAAAZxGIMAAAAAAgAAAAaxGIMAAAAAAcAAAAbxGIMAAAAAAYAAAAcxGIMAAAAAAUAAAAdxGIMAAAAAAQAAAAexGIMAAAAAAMAAAAfxGIMAAAAAAIAAAAgxGIMAAAAAAEAAAABAcRiDAAAAAABAAAAAAAAAOEBAAAAAAAACzwJimPEAZz0EogWy+jjd6sl4A+r0UsE5m5V/vNxdDAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA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',
      'base64',
    ],
    owner: 'Vote111111111111111111111111111111111111111',
    executable: false,
    rentEpoch: 361,
    space: 3731,
  },
};

const SLOTS_PER_EPOCH: bigint = 432000n;
const LAMPORTS_PER_SOL: number = 1_000_000_000;

class BanksConnection {
  constructor(client: BanksClient, payer: Keypair) {
    this.client = client;
    this.payer = payer;
  }

  async getMinimumBalanceForRentExemption(dataLen: number): Promise<number> {
    const rent = await this.client.getRent();
    return Number(rent.minimumBalance(BigInt(dataLen)));
  }

  async getStakeMinimumDelegation() {
    const transaction = new Transaction();
    transaction.add(
      new TransactionInstruction({
        programId: StakeProgram.programId,
        keys: [],
        data: Buffer.from([13, 0, 0, 0]),
      }),
    );
    transaction.recentBlockhash = (await this.client.getLatestBlockhash())[0];
    transaction.feePayer = this.payer.publicKey;
    transaction.sign(this.payer);

    const res = await this.client.simulateTransaction(transaction);
    const data = Array.from(res.inner.meta.returnData.data);
    const minimumDelegation = data[0] + (data[1] << 8) + (data[2] << 16) + (data[3] << 24);

    return { value: minimumDelegation };
  }

  async getAccountInfo(address: PublicKey, commitment?: string): Promise<AccountInfo<Buffer>> {
    const account = await this.client.getAccount(address, commitment);
    if (account) {
      account.data = Buffer.from(account.data);
    }
    return account;
  }
}

async function startWithContext(authorizedWithdrawer?: PublicKey) {
  const voteAccountData = Uint8Array.from(atob(voteAccount.account.data[0]), (c) =>
    c.charCodeAt(0),
  );

  if (authorizedWithdrawer != null) {
    voteAccountData.set(authorizedWithdrawer.toBytes(), 36);
  }

  return await start(
    [
      {
        name: Buffer.from('spl_single_pool').toString('utf-8'),
        programId: SinglePoolProgram.programId,
      },
      {
        name: Buffer.from('mpl_token_metadata').toString('utf-8'),
        programId: MPL_METADATA_PROGRAM_ID,
      },
    ],
    [
      {
        address: new PublicKey(voteAccount.pubkey),
        info: {
          lamports: voteAccount.account.lamports,
          data: voteAccountData,
          owner: VoteProgram.programId,
          executable: false,
        },
      },
    ],
    undefined,
    undefined,
    // stake_raise_minimum_delegation_to_1_sol::id()
    [new PublicKey('9onWzzvCzNC2jfhxxeqRgs5q7nFAAKpCUvkj6T6GJK9i')],
  );
}

async function processTransaction(
  context: ProgramTestContext,
  transaction: Transaction,
  signers = [],
) {
  transaction.recentBlockhash = context.lastBlockhash;
  transaction.feePayer = context.payer.publicKey;
  transaction.sign(...[context.payer].concat(signers));
  return context.banksClient.processTransaction(transaction);
}

async function createAndDelegateStakeAccount(
  context: ProgramTestContext,
  voteAccountAddress: PublicKey,
): Promise<PublicKey> {
  const connection = new BanksConnection(context.banksClient, context.payer);
  let userStakeAccount = new Keypair();

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  let transaction = StakeProgram.createAccount({
    authorized: new Authorized(context.payer.publicKey, context.payer.publicKey),
    fromPubkey: context.payer.publicKey,
    lamports: stakeRent + minimumDelegation,
    stakePubkey: userStakeAccount.publicKey,
  });
  await processTransaction(context, transaction, [userStakeAccount]);
  userStakeAccount = userStakeAccount.publicKey;

  transaction = StakeProgram.delegate({
    authorizedPubkey: context.payer.publicKey,
    stakePubkey: userStakeAccount,
    votePubkey: voteAccountAddress,
  });
  await processTransaction(context, transaction);

  return userStakeAccount;
}

test('initialize', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const onrampAddress = await findPoolOnRampAddress(SinglePoolProgram.programId, poolAddress);

  // initialize pool
  const transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.truthy(await client.getAccount(onrampAddress), 'onramp has been created');
  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('replenish pool', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);

  // initialize pool
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  const slot = await client.getSlot();
  context.warpToSlot(slot + SLOTS_PER_EPOCH);

  // replenish pool
  transaction = await SinglePoolProgram.replenishPool(voteAccountAddress);

  // NOTE we cannot test executing this because bankrun latest is on 1.18
  // maybe someday
  //await processTransaction(context, transaction);
  t.true(true);
});

test('deposit', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const poolStakeAddress = await findPoolStakeAddress(SinglePoolProgram.programId, poolAddress);
  const userStakeAccount = await createAndDelegateStakeAccount(context, voteAccountAddress);

  // initialize pool
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  const slot = await client.getSlot();
  context.warpToSlot(slot + SLOTS_PER_EPOCH);

  // deposit
  transaction = await SinglePoolProgram.deposit({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    userStakeAccount,
  });
  await processTransaction(context, transaction);

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  const poolStakeAccount = await client.getAccount(poolStakeAddress);
  t.is(
    poolStakeAccount.lamports,
    LAMPORTS_PER_SOL + minimumDelegation + stakeRent,
    'stake has been deposited',
  );
});

test('deposit from default', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const poolStakeAddress = await findPoolStakeAddress(SinglePoolProgram.programId, poolAddress);

  // create default account
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  let transaction = await SinglePoolProgram.createAndDelegateUserStake(
    connection,
    voteAccountAddress,
    payer.publicKey,
    minimumDelegation,
  );
  await processTransaction(context, transaction);

  // initialize pool
  transaction = await SinglePoolProgram.initialize(connection, voteAccountAddress, payer.publicKey);
  await processTransaction(context, transaction);

  const slot = await client.getSlot();
  context.warpToSlot(slot + SLOTS_PER_EPOCH);

  // deposit
  transaction = await SinglePoolProgram.deposit({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    depositFromDefaultAccount: true,
  });
  await processTransaction(context, transaction);

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const poolStakeAccount = await client.getAccount(poolStakeAddress);
  t.is(
    poolStakeAccount.lamports,
    LAMPORTS_PER_SOL + minimumDelegation + stakeRent,
    'stake has been deposited',
  );
});

test('withdraw', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const poolStakeAddress = await findPoolStakeAddress(SinglePoolProgram.programId, poolAddress);
  const depositAccount = await createAndDelegateStakeAccount(context, voteAccountAddress);

  // initialize pool
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  const slot = await client.getSlot();
  context.warpToSlot(slot + SLOTS_PER_EPOCH);

  // deposit
  transaction = await SinglePoolProgram.deposit({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    userStakeAccount: depositAccount,
  });
  await processTransaction(context, transaction);

  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  const poolStakeAccount = await client.getAccount(poolStakeAddress);
  t.true(poolStakeAccount.lamports > minimumDelegation * 2, 'stake has been deposited');

  // withdraw
  const withdrawAccount = new Keypair();
  transaction = await SinglePoolProgram.withdraw({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    userStakeAccount: withdrawAccount.publicKey,
    tokenAmount: minimumDelegation,
    createStakeAccount: true,
  });
  await processTransaction(context, transaction, [withdrawAccount]);

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const userStakeAccount = await client.getAccount(withdrawAccount.publicKey);
  t.is(userStakeAccount.lamports, minimumDelegation + stakeRent, 'stake has been withdrawn');
});

test('create metadata', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);

  // initialize pool without metadata
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
    true,
  );
  await processTransaction(context, transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.falsy(
    await client.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has not been created',
  );

  // create metadata
  transaction = await SinglePoolProgram.createTokenMetadata(poolAddress, payer.publicKey);
  await processTransaction(context, transaction);

  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('update metadata', async (t) => {
  const authorizedWithdrawer = new Keypair();

  const context = await startWithContext(authorizedWithdrawer.publicKey);
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const poolMintAddress = await findPoolMintAddress(SinglePoolProgram.programId, poolAddress);
  const poolMetadataAddress = findMplMetadataAddress(poolMintAddress);

  // initialize pool
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  // update metadata
  const newName = 'hana wuz here';
  transaction = await SinglePoolProgram.updateTokenMetadata(
    voteAccountAddress,
    authorizedWithdrawer.publicKey,
    newName,
    '',
  );
  await processTransaction(context, transaction, [authorizedWithdrawer]);

  const metadataAccount = await client.getAccount(poolMetadataAddress);
  t.true(
    new TextDecoder('ascii').decode(metadataAccount.data).indexOf(newName) > -1,
    'metadata name has been updated',
  );
});

test('get vote account address', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const payer = context.payer;
  const connection = new BanksConnection(client, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);

  // initialize pool
  const transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);

  const chainVoteAccount = await getVoteAccountAddressForPool(connection, poolAddress);
  t.true(chainVoteAccount.equals(voteAccountAddress), 'got correct vote account');
});

test('default account address', async (t) => {
  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const owner = new PublicKey('GtaYCtXWCrciizttN5mx9P38niTQPGWpfu6DnSgAr3Cj');
  const expectedDefault = new PublicKey('BbfrNeJrd82cSFsULXT9zG8SvLLB8WsTc1gQsDFy3Sed');

  const actualDefault = await findDefaultDepositAccountAddress(
    await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress),
    owner,
  );

  t.true(actualDefault.equals(expectedDefault), 'got correct default account address');
});
