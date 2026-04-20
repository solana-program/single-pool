import test from 'ava';
import path from 'node:path';
import { LiteSVM, FailedTransactionMetadata, StakeHistoryEntry } from 'litesvm';
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  Authorized,
  TransactionInstruction,
  StakeProgram,
  VoteProgram,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
  getVoteAccountAddressForPool,
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

const LAMPORTS_PER_SOL: number = 1_000_000_000;
const STAKE_HISTORY_ENTRY = new StakeHistoryEntry(
  BigInt(LAMPORTS_PER_SOL) * 1000n,
  BigInt(LAMPORTS_PER_SOL) * 10n,
  BigInt(LAMPORTS_PER_SOL) * 10n,
);

type LiteContext = {
  svm: LiteSVM;
  payer: Keypair;
  advanceEpoch(): void;
};

class LiteConnection {
  constructor(svm: LiteSVM, payer: Keypair) {
    this.svm = svm;
    this.payer = payer;
  }

  async getMinimumBalanceForRentExemption(dataLen: number): Promise<number> {
    const rent = await this.svm.getRent();
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
    transaction.recentBlockhash = this.svm.latestBlockhash();
    transaction.feePayer = this.payer.publicKey;
    transaction.sign(this.payer);

    const res = await this.svm.simulateTransaction(transaction);
    const data = Array.from(res.meta().returnData().data());
    const minimumDelegation = data[0] + (data[1] << 8) + (data[2] << 16) + (data[3] << 24);

    return { value: minimumDelegation };
  }

  async getAccountInfo(address: PublicKey, _commitment?: string): Promise<AccountInfo<Buffer>> {
    void _commitment;
    const account = await this.svm.getAccount(address);
    if (account) {
      account.data = Buffer.from(account.data);
    }
    return account;
  }
}

async function startWithContext(authorizedWithdrawer?: PublicKey): Promise<LiteContext> {
  const voteAccountData = Uint8Array.from(atob(voteAccount.account.data[0]), (c) =>
    c.charCodeAt(0),
  );

  if (authorizedWithdrawer != null) {
    voteAccountData.set(authorizedWithdrawer.toBytes(), 36);
  }

  const svm = new LiteSVM();
  const payer = Keypair.generate();

  svm.airdrop(payer.publicKey, 10_000n * BigInt(LAMPORTS_PER_SOL));

  svm.addProgramFromFile(
    SinglePoolProgram.programId,
    path.resolve(process.cwd(), 'tests', 'fixtures', 'spl_single_pool.so'),
  );
  svm.addProgramFromFile(
    MPL_METADATA_PROGRAM_ID,
    path.resolve(process.cwd(), 'tests', 'fixtures', 'mpl_token_metadata.so'),
  );

  svm.setAccount(new PublicKey(voteAccount.pubkey), {
    lamports: voteAccount.account.lamports,
    data: voteAccountData,
    owner: VoteProgram.programId,
    executable: false,
    rentEpoch: 0,
  });

  const schedule = svm.getEpochSchedule();
  const clock = svm.getClock();
  const history = svm.getStakeHistory();

  clock.slot = schedule.firstNormalSlot + 1n;
  clock.epoch = schedule.firstNormalEpoch;
  clock.leaderScheduleEpoch = schedule.firstNormalEpoch;

  for (let epoch = 0n; epoch < schedule.firstNormalEpoch; epoch += 1n) {
    history.add(epoch, STAKE_HISTORY_ENTRY);
  }

  svm.setClock(clock);
  svm.setStakeHistory(history);

  return {
    svm,
    payer,
    advanceEpoch() {
      const schedule = svm.getEpochSchedule();
      const clock = svm.getClock();
      const history = svm.getStakeHistory();

      history.add(clock.epoch, STAKE_HISTORY_ENTRY);
      clock.slot += schedule.slotsPerEpoch;
      clock.epoch += 1n;
      clock.leaderScheduleEpoch = clock.epoch;

      svm.setClock(clock);
      svm.setStakeHistory(history);
    },
  };
}

async function processTransaction(context: LiteContext, transaction: Transaction, signers = []) {
  transaction.recentBlockhash = context.svm.latestBlockhash();
  transaction.feePayer = context.payer.publicKey;
  transaction.sign(...[context.payer].concat(signers));

  const res = context.svm.sendTransaction(transaction);
  if (res instanceof FailedTransactionMetadata) {
    throw new Error(`${res.err().toString()}\n${res.meta().prettyLogs()}`);
  }
  return res;
}

async function createAndDelegateStakeAccount(
  context: LiteContext,
  voteAccountAddress: PublicKey,
): Promise<PublicKey> {
  const connection = new LiteConnection(context.svm, context.payer);
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
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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

  t.truthy(svm.getAccount(poolAddress), 'pool has been created');
  t.truthy(svm.getAccount(onrampAddress), 'onramp has been created');
  t.truthy(
    svm.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('replenish pool', async (t) => {
  const context = await startWithContext();
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = await findPoolAddress(SinglePoolProgram.programId, voteAccountAddress);
  const poolStakeAddress = await findPoolStakeAddress(SinglePoolProgram.programId, poolAddress);
  const poolOnrampAddress = await findPoolOnRampAddress(SinglePoolProgram.programId, poolAddress);

  // initialize pool
  let transaction = await SinglePoolProgram.initialize(
    connection,
    voteAccountAddress,
    payer.publicKey,
  );
  await processTransaction(context, transaction);
  context.advanceEpoch();

  transaction = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: poolStakeAddress,
      lamports: LAMPORTS_PER_SOL,
    }),
  );
  await processTransaction(context, transaction);

  // replenish pool
  transaction = await SinglePoolProgram.replenishPool(voteAccountAddress);
  await processTransaction(context, transaction);

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const poolOnrampAccount = svm.getAccount(poolOnrampAddress);
  t.is(poolOnrampAccount.lamports, LAMPORTS_PER_SOL + stakeRent, 'lamports have been replenished');
});

test('deposit', async (t) => {
  const context = await startWithContext();
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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
  context.advanceEpoch();

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
  const poolStakeAccount = svm.getAccount(poolStakeAddress);
  t.is(
    poolStakeAccount.lamports,
    LAMPORTS_PER_SOL + minimumDelegation + stakeRent,
    'stake has been deposited',
  );
});

test('withdraw', async (t) => {
  const context = await startWithContext();
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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
  context.advanceEpoch();

  // deposit
  transaction = await SinglePoolProgram.deposit({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    userStakeAccount: depositAccount,
  });
  await processTransaction(context, transaction);

  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  const poolStakeAccount = svm.getAccount(poolStakeAddress);
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
  const userStakeAccount = svm.getAccount(withdrawAccount.publicKey);
  t.is(userStakeAccount.lamports, minimumDelegation + stakeRent, 'stake has been withdrawn');
});

test('create metadata', async (t) => {
  const context = await startWithContext();
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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

  t.truthy(svm.getAccount(poolAddress), 'pool has been created');
  t.falsy(
    svm.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has not been created',
  );

  // create metadata
  transaction = await SinglePoolProgram.createTokenMetadata(poolAddress, payer.publicKey);
  await processTransaction(context, transaction);

  t.truthy(
    svm.getAccount(
      findMplMetadataAddress(await findPoolMintAddress(SinglePoolProgram.programId, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('update metadata', async (t) => {
  const authorizedWithdrawer = new Keypair();

  const context = await startWithContext(authorizedWithdrawer.publicKey);
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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

  const metadataAccount = svm.getAccount(poolMetadataAddress);
  t.true(
    new TextDecoder('ascii').decode(metadataAccount.data).indexOf(newName) > -1,
    'metadata name has been updated',
  );
});

test('get vote account address', async (t) => {
  const context = await startWithContext();
  const svm = context.svm;
  const payer = context.payer;
  const connection = new LiteConnection(svm, payer);

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
