# Single-Validator Stake Pool

Fully permissionless liquid staking.

| Information | Account Address |
| --- | --- |
| Single Pool | `SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE` |

## Overview

The Single-Validator Stake Pool is an onchain program that enables liquid staking with zero fees, no counterparty, and 100% capital efficiency. The program defines a canonical pool for every vote account, which can be initialized permissionlessly, and mints tokens in exchange for stake delegated to its designated validator.

The program also allows permissionless harvesting of Jito tips and other MEV rewards, turning liquid sol paid into the stake account into active stake earning rewards, functionally distributing these earnings to all LST holders just like protocol staking rewards.

Users can only deposit and withdraw active stake, but liquid sol deposit is coming in a future update.

## Security Audits

The Single Pool Program has received three external audits:

* Zellic (2024-01-02)
    - Review commit hash [`ef44df9`](https://github.com/solana-program/single-pool/commit/ef44df985e76a697ee9a8aabb3a223610e4cf1dc)
    - Final report <https://github.com/anza-xyz/security-audits/blob/master/spl/ZellicSinglePoolAudit-2024-01-02.pdf>
* Neodyme (2023-08-08)
    - Review commit hash [`735d729`](https://github.com/solana-program/single-pool/commit/735d7292e35d35101750a4452d2647bdbf848e8b)
    - Final report <https://github.com/anza-xyz/security-audits/blob/master/spl/NeodymeSinglePoolAudit-2023-08-08.pdf>
* Zellic (2023-06-21)
    - Review commit hash [`9dbdc3b`](https://github.com/solana-program/single-pool/commit/9dbdc3bdae31dda1dcb35346aab2d879deecf194)
    - Final report <https://github.com/anza-xyz/security-audits/blob/master/spl/ZellicSinglePoolAudit-2023-06-21.pdf>

## Building and Verifying

To build the Single Pool Program, you can run `cargo-build-sbf` or use the Makefile
command:

```console
cargo build-sbf --manifest-path program/Cargo.toml
make build-sbf-program
```

The BPF program deployed on all clusters is built with [solana-verify](https://solana.com/developers/guides/advanced/verified-builds). It may be verified independently by comparing the output of:

```console
solana-verify get-program-hash -um SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE
```

with:

```console
solana-verify build --library-name program
solana-verify get-executable-hash target/deploy/spl_single_pool.so
```
