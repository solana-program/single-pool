import {
  getAddressCodec,
  Address,
  ReadonlySignerAccount,
  ReadonlyAccount,
  InstructionWithAccounts,
  InstructionWithData,
  WritableAccount,
  WritableSignerAccount,
  Instruction,
  AccountRole,
  getU32Encoder,
  getU64Encoder,
} from '@solana/kit';

import {
  PoolMintAuthorityAddress,
  PoolMintAddress,
  PoolMplAuthorityAddress,
  PoolStakeAuthorityAddress,
  PoolStakeAddress,
  PoolOnRampAddress,
  findMplMetadataAddress,
  findPoolMplAuthorityAddress,
  findPoolAddress,
  VoteAccountAddress,
  PoolAddress,
  findPoolStakeAddress,
  findPoolOnRampAddress,
  findPoolMintAddress,
  findPoolMintAuthorityAddress,
  findPoolStakeAuthorityAddress,
  SINGLE_POOL_PROGRAM_ID,
} from './addresses.js';
import { TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';
import { SYSTEM_PROGRAM_ADDRESS } from '@solana-program/system';
import { STAKE_PROGRAM_ADDRESS } from '@solana-program/stake';
import {
  SYSVAR_RENT_ADDRESS,
  SYSVAR_CLOCK_ADDRESS,
  SYSVAR_STAKE_HISTORY_ADDRESS,
} from '@solana/sysvars';
import { MPL_METADATA_PROGRAM_ID, STAKE_CONFIG_ID } from './internal.js';

type InitializePoolInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      WritableAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      ReadonlyAccount<typeof SYSVAR_RENT_ADDRESS>,
      ReadonlyAccount<typeof SYSVAR_CLOCK_ADDRESS>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ADDRESS>,
      ReadonlyAccount<typeof STAKE_CONFIG_ID>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type ReplenishPoolInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolOnRampAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<typeof SYSVAR_CLOCK_ADDRESS>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ADDRESS>,
      ReadonlyAccount<typeof STAKE_CONFIG_ID>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type DepositStakeInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      ReadonlyAccount<PoolOnRampAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      WritableAccount<Address>, // user stake
      WritableAccount<Address>, // user token
      WritableAccount<Address>, // user lamport
      ReadonlyAccount<typeof SYSVAR_CLOCK_ADDRESS>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ADDRESS>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type WithdrawStakeInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      ReadonlyAccount<PoolOnRampAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      WritableAccount<Address>, // user stake
      WritableAccount<Address>, // user token
      ReadonlyAccount<typeof SYSVAR_CLOCK_ADDRESS>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type CreateTokenMetadataInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      ReadonlyAccount<PoolMintAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      ReadonlyAccount<PoolMplAuthorityAddress>,
      WritableSignerAccount<Address>, // mpl payer
      WritableAccount<Address>, // mpl account
      ReadonlyAccount<typeof MPL_METADATA_PROGRAM_ID>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type UpdateTokenMetadataInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      ReadonlyAccount<PoolAddress>,
      ReadonlyAccount<PoolMplAuthorityAddress>,
      ReadonlySignerAccount<Address>, // authorized withdrawer
      WritableAccount<Address>, // mpl account
      ReadonlyAccount<typeof MPL_METADATA_PROGRAM_ID>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type InitializeOnRampInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolOnRampAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<typeof SYSVAR_RENT_ADDRESS>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
    ]
  > &
  InstructionWithData<Uint8Array>;

type DepositSolInstruction = Instruction<typeof SINGLE_POOL_PROGRAM_ID> &
  InstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolOnRampAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      WritableSignerAccount<Address>, // user lamport
      WritableAccount<Address>, // user token
      ReadonlyAccount<typeof SYSVAR_CLOCK_ADDRESS>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ADDRESS>,
      ReadonlyAccount<typeof STAKE_CONFIG_ID>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ADDRESS>,
      ReadonlyAccount<typeof SINGLE_POOL_PROGRAM_ID>,
    ]
  > &
  InstructionWithData<Uint8Array>;

const enum SinglePoolInstructionType {
  InitializePool = 0,
  ReplenishPool,
  DepositStake,
  WithdrawStake,
  CreateTokenMetadata,
  UpdateTokenMetadata,
  InitializeOnRamp,
  DepositSol,
}

export const SinglePoolInstruction = {
  initializePool: initializePoolInstruction,
  replenishPool: replenishPoolInstruction,
  depositStake: depositStakeInstruction,
  withdrawStake: withdrawStakeInstruction,
  createTokenMetadata: createTokenMetadataInstruction,
  updateTokenMetadata: updateTokenMetadataInstruction,
  initializeOnRamp: initializeOnRampInstruction,
  depositSol: depositSolInstruction,
};

export async function initializePoolInstruction(
  voteAccount: VoteAccountAddress,
): Promise<InitializePoolInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const [stake, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.InitializePool]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.WRITABLE },
      { address: stake, role: AccountRole.WRITABLE },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: SYSVAR_RENT_ADDRESS, role: AccountRole.READONLY },
      { address: SYSVAR_CLOCK_ADDRESS, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function replenishPoolInstruction(
  voteAccount: VoteAccountAddress,
): Promise<ReplenishPoolInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const [stake, onramp, stakeAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolOnRampAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.ReplenishPool]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: onramp, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: SYSVAR_CLOCK_ADDRESS, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function depositStakeInstruction(
  pool: PoolAddress,
  userStakeAccount: Address,
  userTokenAccount: Address,
  userLamportAccount: Address,
): Promise<DepositStakeInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const [stake, onramp, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolOnRampAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.DepositStake]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: onramp, role: AccountRole.READONLY },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: userStakeAccount, role: AccountRole.WRITABLE },
      { address: userTokenAccount, role: AccountRole.WRITABLE },
      { address: userLamportAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ADDRESS, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ADDRESS, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function withdrawStakeInstruction(
  pool: PoolAddress,
  userStakeAccount: Address,
  userStakeAuthority: Address,
  userTokenAccount: Address,
  tokenAmount: bigint,
): Promise<WithdrawStakeInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const [stake, onramp, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolOnRampAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const { encode } = getAddressCodec();
  const data = new Uint8Array([
    SinglePoolInstructionType.WithdrawStake,
    ...encode(userStakeAuthority),
    ...getU64Encoder().encode(tokenAmount),
  ]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: onramp, role: AccountRole.READONLY },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: userStakeAccount, role: AccountRole.WRITABLE },
      { address: userTokenAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ADDRESS, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function createTokenMetadataInstruction(
  pool: PoolAddress,
  payer: Address,
): Promise<CreateTokenMetadataInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const mint = await findPoolMintAddress(programAddress, pool);
  const [mintAuthority, mplAuthority, mplMetadata] = await Promise.all([
    findPoolMintAuthorityAddress(programAddress, pool),
    findPoolMplAuthorityAddress(programAddress, pool),
    findMplMetadataAddress(mint),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.CreateTokenMetadata]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: mint, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: mplAuthority, role: AccountRole.READONLY },
      { address: payer, role: AccountRole.WRITABLE_SIGNER },
      { address: mplMetadata, role: AccountRole.WRITABLE },
      { address: MPL_METADATA_PROGRAM_ID, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function updateTokenMetadataInstruction(
  voteAccount: VoteAccountAddress,
  authorizedWithdrawer: Address,
  tokenName: string,
  tokenSymbol: string,
  tokenUri?: string,
): Promise<UpdateTokenMetadataInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  tokenUri = tokenUri || '';

  if (tokenName.length > 32) {
    throw 'maximum token name length is 32 characters';
  }

  if (tokenSymbol.length > 10) {
    throw 'maximum token symbol length is 10 characters';
  }

  if (tokenUri.length > 200) {
    throw 'maximum token uri length is 200 characters';
  }

  const pool = await findPoolAddress(programAddress, voteAccount);
  const [mint, mplAuthority] = await Promise.all([
    findPoolMintAddress(programAddress, pool),
    findPoolMplAuthorityAddress(programAddress, pool),
  ]);
  const mplMetadata = await findMplMetadataAddress(mint);

  const text = new TextEncoder();
  const data = new Uint8Array([
    SinglePoolInstructionType.UpdateTokenMetadata,
    ...getU32Encoder().encode(tokenName.length),
    ...text.encode(tokenName),
    ...getU32Encoder().encode(tokenSymbol.length),
    ...text.encode(tokenSymbol),
    ...getU32Encoder().encode(tokenUri.length),
    ...text.encode(tokenUri),
  ]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.READONLY },
      { address: mplAuthority, role: AccountRole.READONLY },
      { address: authorizedWithdrawer, role: AccountRole.READONLY_SIGNER },
      { address: mplMetadata, role: AccountRole.WRITABLE },
      { address: MPL_METADATA_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function initializeOnRampInstruction(
  pool: PoolAddress,
): Promise<InitializeOnRampInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const [onramp, stakeAuthority] = await Promise.all([
    findPoolOnRampAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.InitializeOnRamp]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: onramp, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: SYSVAR_RENT_ADDRESS, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function depositSolInstruction(
  voteAccount: VoteAccountAddress,
  userLamportAccount: Address,
  userTokenAccount: Address,
  lamports: bigint,
): Promise<DepositSolInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const [stake, onramp, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolOnRampAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([
    SinglePoolInstructionType.DepositSol,
    ...getU64Encoder().encode(lamports),
  ]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: onramp, role: AccountRole.WRITABLE },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: userLamportAccount, role: AccountRole.WRITABLE_SIGNER },
      { address: userTokenAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ADDRESS, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ADDRESS, role: AccountRole.READONLY },
      { address: SINGLE_POOL_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}
