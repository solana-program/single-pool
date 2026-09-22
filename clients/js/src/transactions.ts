import {
  Address,
  pipe,
  InstructionPlan,
  assertIsSingleTransactionPlan,
  createTransactionPlanner,
  parallelInstructionPlan,
  sequentialInstructionPlan,
  GetAccountInfoApi,
  GetMinimumBalanceForRentExemptionApi,
  GetStakeMinimumDelegationApi,
  Rpc,
  appendTransactionMessageInstruction,
  createTransactionMessage,
  setTransactionMessageFeePayer,
  TransactionVersion,
  TransactionMessage,
  createNoopSigner,
} from '@solana/kit';

import {
  findPoolAddress,
  VoteAccountAddress,
  PoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  findPoolOnRampAddress,
  findPoolMintAuthorityAddress,
  findPoolStakeAuthorityAddress,
  SINGLE_POOL_PROGRAM_ID,
} from './addresses.js';
import {
  initializePoolInstruction,
  replenishPoolInstruction,
  depositStakeInstruction,
  withdrawStakeInstruction,
  createTokenMetadataInstruction,
  updateTokenMetadataInstruction,
  initializeOnRampInstruction,
  depositSolInstruction,
} from './instructions.js';
import {
  getApproveInstruction,
  getCreateAssociatedTokenInstruction,
  findAssociatedTokenPda,
  getMintSize,
  TOKEN_PROGRAM_ADDRESS,
} from '@solana-program/token';
import { getCreateAccountInstruction, getTransferSolInstruction } from '@solana-program/system';
import {
  getAuthorizeInstruction,
  StakeAuthorize,
  STAKE_PROGRAM_ADDRESS,
} from '@solana-program/stake';
import { STAKE_ACCOUNT_SIZE } from './internal.js';

interface DepositParams {
  rpc: Rpc<GetAccountInfoApi & GetMinimumBalanceForRentExemptionApi & GetStakeMinimumDelegationApi>;
  pool: PoolAddress;
  userWallet: Address;
  userStakeAccount: Address;
  userTokenAccount?: Address;
  userLamportAccount?: Address;
  userWithdrawAuthority?: Address;
}

interface WithdrawParams {
  rpc: Rpc<GetMinimumBalanceForRentExemptionApi & GetStakeMinimumDelegationApi>;
  pool: PoolAddress;
  userWallet: Address;
  userStakeAccount: Address;
  tokenAmount: bigint;
  createStakeAccount?: boolean;
  userStakeAuthority?: Address;
  userTokenAccount?: Address;
  userTokenAuthority?: Address;
}

interface DepositSolParams {
  rpc: Rpc<GetAccountInfoApi>;
  voteAccount: VoteAccountAddress;
  userWallet: Address;
  lamports: bigint;
  userTokenAccount?: Address;
}

export const SINGLE_POOL_ACCOUNT_SIZE = 33n;

export const SinglePoolProgram = {
  programAddress: SINGLE_POOL_PROGRAM_ID,
  space: SINGLE_POOL_ACCOUNT_SIZE,
  initialize: initializeTransaction,
  replenishPool: replenishPoolTransaction,
  deposit: depositTransaction,
  withdraw: withdrawTransaction,
  createTokenMetadata: createTokenMetadataTransaction,
  updateTokenMetadata: updateTokenMetadataTransaction,
  initializeOnRamp: initializeOnRampTransaction,
  depositSol: depositSolTransaction,
};

async function getInitializeInstructionPlan(
  rpc: Rpc<GetMinimumBalanceForRentExemptionApi & GetStakeMinimumDelegationApi>,
  voteAccount: VoteAccountAddress,
  payer: Address,
  skipMetadata = false,
): Promise<InstructionPlan> {
  const pool = await findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccount);
  const [
    stake,
    mint,
    onramp,
    poolRent,
    stakeRent,
    mintRent,
    minimumDelegationObj,
    initializePool,
    initializeOnRamp,
  ] = await Promise.all([
    findPoolStakeAddress(SINGLE_POOL_PROGRAM_ID, pool),
    findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
    findPoolOnRampAddress(SINGLE_POOL_PROGRAM_ID, pool),
    rpc.getMinimumBalanceForRentExemption(SINGLE_POOL_ACCOUNT_SIZE).send(),
    rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
    rpc.getMinimumBalanceForRentExemption(BigInt(getMintSize())).send(),
    rpc.getStakeMinimumDelegation().send(),
    initializePoolInstruction(voteAccount),
    initializeOnRampInstruction(pool),
  ]);
  const lamportsPerSol = 1_000_000_000n;
  const minimumPoolBalance =
    minimumDelegationObj.value > lamportsPerSol ? minimumDelegationObj.value : lamportsPerSol;
  const payerSigner = createNoopSigner(payer);

  return sequentialInstructionPlan([
    parallelInstructionPlan([
      getTransferSolInstruction({ source: payerSigner, destination: pool, amount: poolRent }),
      getTransferSolInstruction({
        source: payerSigner,
        destination: stake,
        amount: stakeRent + minimumPoolBalance,
      }),
      getTransferSolInstruction({ source: payerSigner, destination: onramp, amount: stakeRent }),
      getTransferSolInstruction({ source: payerSigner, destination: mint, amount: mintRent }),
    ]),
    initializePool,
    initializeOnRamp,
    ...(skipMetadata ? [] : [await createTokenMetadataInstruction(pool, payer)]),
  ]);
}

export async function initializeTransaction(
  rpc: Rpc<GetMinimumBalanceForRentExemptionApi & GetStakeMinimumDelegationApi>,
  voteAccount: VoteAccountAddress,
  payer: Address,
  skipMetadata = false,
): Promise<TransactionMessage> {
  const transactionPlanner = createTransactionPlanner({
    createTransactionMessage: () =>
      pipe(createTransactionMessage({ version: 0 }), (m) =>
        setTransactionMessageFeePayer(payer, m),
      ),
  });

  const instructionPlan = await getInitializeInstructionPlan(rpc, voteAccount, payer, skipMetadata);
  const transactionPlan = await transactionPlanner(instructionPlan);
  assertIsSingleTransactionPlan(transactionPlan);
  return transactionPlan.message;
}

export async function replenishPoolTransaction(
  voteAccount: VoteAccountAddress,
): Promise<TransactionMessage> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionMessageInstruction(
    await replenishPoolInstruction(voteAccount),
    transaction,
  );

  return transaction;
}

export async function depositTransaction(params: DepositParams) {
  const { rpc, pool, userWallet, userStakeAccount } = params;

  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const [mint, poolStakeAuthority] = await Promise.all([
    findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
    findPoolStakeAuthorityAddress(SINGLE_POOL_PROGRAM_ID, pool),
  ]);

  const [userAssociatedTokenAccount] = await findAssociatedTokenPda({
    owner: userWallet,
    mint,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  const userTokenAccount = params.userTokenAccount || userAssociatedTokenAccount;
  const userLamportAccount = params.userLamportAccount || userWallet;
  const userWithdrawAuthority = params.userWithdrawAuthority || userWallet;

  if (
    userTokenAccount == userAssociatedTokenAccount &&
    (await rpc.getAccountInfo(userAssociatedTokenAccount).send()) == null
  ) {
    transaction = appendTransactionMessageInstruction(
      getCreateAssociatedTokenInstruction({
        payer: createNoopSigner(userWallet),
        ata: userAssociatedTokenAccount,
        owner: userWallet,
        mint,
      }),
      transaction,
    );
  }

  transaction = appendTransactionMessageInstruction(
    getAuthorizeInstruction({
      stake: userStakeAccount,
      authority: createNoopSigner(userWithdrawAuthority),
      arg0: poolStakeAuthority,
      arg1: StakeAuthorize.Staker,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    getAuthorizeInstruction({
      stake: userStakeAccount,
      authority: createNoopSigner(userWithdrawAuthority),
      arg0: poolStakeAuthority,
      arg1: StakeAuthorize.Withdrawer,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    await depositStakeInstruction(pool, userStakeAccount, userTokenAccount, userLamportAccount),
    transaction,
  );

  return transaction;
}

export async function withdrawTransaction(params: WithdrawParams) {
  const { rpc, pool, userWallet, userStakeAccount, tokenAmount, createStakeAccount } = params;

  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const poolMintAuthority = await findPoolMintAuthorityAddress(SINGLE_POOL_PROGRAM_ID, pool);

  const userStakeAuthority = params.userStakeAuthority || userWallet;
  const userTokenAccount =
    params.userTokenAccount ||
    (
      await findAssociatedTokenPda({
        owner: userWallet,
        mint: await findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
      })
    )[0];
  const userTokenAuthority = params.userTokenAuthority || userWallet;

  if (createStakeAccount) {
    transaction = appendTransactionMessageInstruction(
      getCreateAccountInstruction({
        payer: createNoopSigner(userWallet),
        newAccount: createNoopSigner(userStakeAccount),
        lamports: await rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
        space: STAKE_ACCOUNT_SIZE,
        programAddress: STAKE_PROGRAM_ADDRESS,
      }),
      transaction,
    );
  }

  transaction = appendTransactionMessageInstruction(
    getApproveInstruction({
      source: userTokenAccount,
      delegate: poolMintAuthority,
      owner: createNoopSigner(userTokenAuthority),
      amount: tokenAmount,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    await withdrawStakeInstruction(
      pool,
      userStakeAccount,
      userStakeAuthority,
      userTokenAccount,
      tokenAmount,
    ),
    transaction,
  );

  return transaction;
}

export async function createTokenMetadataTransaction(
  pool: PoolAddress,
  payer: Address,
): Promise<TransactionMessage> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionMessageInstruction(
    await createTokenMetadataInstruction(pool, payer),
    transaction,
  );

  return transaction;
}

export async function updateTokenMetadataTransaction(
  voteAccount: VoteAccountAddress,
  authorizedWithdrawer: Address,
  name: string,
  symbol: string,
  uri?: string,
): Promise<TransactionMessage> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionMessageInstruction(
    await updateTokenMetadataInstruction(voteAccount, authorizedWithdrawer, name, symbol, uri),
    transaction,
  );

  return transaction;
}

export async function initializeOnRampTransaction(
  rpc: Rpc<GetMinimumBalanceForRentExemptionApi & GetStakeMinimumDelegationApi>,
  pool: PoolAddress,
  payer: Address,
): Promise<TransactionMessage> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const [onramp, stakeRent] = await Promise.all([
    findPoolOnRampAddress(SINGLE_POOL_PROGRAM_ID, pool),
    rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
  ]);

  transaction = appendTransactionMessageInstruction(
    getTransferSolInstruction({
      source: createNoopSigner(payer),
      destination: onramp,
      amount: stakeRent,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    await initializeOnRampInstruction(pool),
    transaction,
  );

  return transaction;
}

export async function depositSolTransaction(params: DepositSolParams) {
  const { rpc, voteAccount, userWallet, lamports } = params;

  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const pool = await findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccount);
  const mint = await findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool);

  const [userAssociatedTokenAccount] = await findAssociatedTokenPda({
    owner: userWallet,
    mint,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  const userTokenAccount = params.userTokenAccount || userAssociatedTokenAccount;

  if (
    userTokenAccount == userAssociatedTokenAccount &&
    (await rpc.getAccountInfo(userAssociatedTokenAccount).send()) == null
  ) {
    transaction = appendTransactionMessageInstruction(
      getCreateAssociatedTokenInstruction({
        payer: createNoopSigner(userWallet),
        ata: userAssociatedTokenAccount,
        owner: userWallet,
        mint,
      }),
      transaction,
    );
  }

  // NOTE in our rust instruction builder, we transfer lamports to an escrow account.
  // this allows us to give greater assurance to the end user that their signing
  // authority cannot be misused by our benevolent, yet untrusted, program.
  // unfortunately this is not possible in js middleware but dapps may wish to consider
  // doing similar by injecting a system transfer between these two instructions

  transaction = appendTransactionMessageInstruction(
    await depositSolInstruction(voteAccount, userWallet, userTokenAccount, lamports),
    transaction,
  );

  return transaction;
}
