import { Address } from '@solana/addresses';
import { pipe } from '@solana/functional';
import {
  InstructionPlan,
  assertIsSingleTransactionPlan,
  createTransactionPlanner,
  parallelInstructionPlan,
  sequentialInstructionPlan,
} from '@solana/instruction-plans';
import {
  GetAccountInfoApi,
  GetMinimumBalanceForRentExemptionApi,
  GetStakeMinimumDelegationApi,
} from '@solana/rpc-api';
import { Rpc } from '@solana/rpc-spec';
import {
  appendTransactionMessageInstruction,
  createTransactionMessage,
  setTransactionMessageFeePayer,
  TransactionVersion,
  TransactionMessage,
} from '@solana/transaction-messages';

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
} from './instructions.js';
import {
  STAKE_PROGRAM_ID,
  STAKE_ACCOUNT_SIZE,
  MINT_SIZE,
  StakeInstruction,
  SystemInstruction,
  TokenInstruction,
  StakeAuthorizationType,
  getAssociatedTokenAddress,
} from './quarantine.js';

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
    rpc.getMinimumBalanceForRentExemption(MINT_SIZE).send(),
    rpc.getStakeMinimumDelegation().send(),
    initializePoolInstruction(voteAccount),
    initializeOnRampInstruction(pool),
  ]);
  const lamportsPerSol = 1_000_000_000n;
  const minimumPoolBalance =
    minimumDelegationObj.value > lamportsPerSol ? minimumDelegationObj.value : lamportsPerSol;

  return sequentialInstructionPlan([
    parallelInstructionPlan([
      SystemInstruction.transfer({ from: payer, to: pool, lamports: poolRent }),
      SystemInstruction.transfer({
        from: payer,
        to: stake,
        lamports: stakeRent + minimumPoolBalance,
      }),
      SystemInstruction.transfer({ from: payer, to: onramp, lamports: stakeRent }),
      SystemInstruction.transfer({ from: payer, to: mint, lamports: mintRent }),
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

  const userAssociatedTokenAccount = await getAssociatedTokenAddress(mint, userWallet);
  const userTokenAccount = params.userTokenAccount || userAssociatedTokenAccount;
  const userLamportAccount = params.userLamportAccount || userWallet;
  const userWithdrawAuthority = params.userWithdrawAuthority || userWallet;

  if (
    userTokenAccount == userAssociatedTokenAccount &&
    (await rpc.getAccountInfo(userAssociatedTokenAccount).send()) == null
  ) {
    transaction = appendTransactionMessageInstruction(
      TokenInstruction.createAssociatedTokenAccount({
        payer: userWallet,
        associatedAccount: userAssociatedTokenAccount,
        owner: userWallet,
        mint,
      }),
      transaction,
    );
  }

  transaction = appendTransactionMessageInstruction(
    StakeInstruction.authorize({
      stakeAccount: userStakeAccount,
      authorized: userWithdrawAuthority,
      newAuthorized: poolStakeAuthority,
      authorizationType: StakeAuthorizationType.Staker,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    StakeInstruction.authorize({
      stakeAccount: userStakeAccount,
      authorized: userWithdrawAuthority,
      newAuthorized: poolStakeAuthority,
      authorizationType: StakeAuthorizationType.Withdrawer,
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
    (await getAssociatedTokenAddress(
      await findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
      userWallet,
    ));
  const userTokenAuthority = params.userTokenAuthority || userWallet;

  if (createStakeAccount) {
    transaction = appendTransactionMessageInstruction(
      SystemInstruction.createAccount({
        from: userWallet,
        lamports: await rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
        newAccount: userStakeAccount,
        programAddress: STAKE_PROGRAM_ID,
        space: STAKE_ACCOUNT_SIZE,
      }),
      transaction,
    );
  }

  transaction = appendTransactionMessageInstruction(
    TokenInstruction.approve({
      account: userTokenAccount,
      delegate: poolMintAuthority,
      owner: userTokenAuthority,
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
    SystemInstruction.transfer({
      from: payer,
      to: onramp,
      lamports: stakeRent,
    }),
    transaction,
  );

  transaction = appendTransactionMessageInstruction(
    await initializeOnRampInstruction(pool),
    transaction,
  );

  return transaction;
}
