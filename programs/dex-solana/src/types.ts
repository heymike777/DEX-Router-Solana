import { PublicKey } from '@solana/web3.js';

// Program IDs
export const OKX_DEX_ROUTER_PROGRAM_ID = new PublicKey('6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma');
export const RAYDIUM_SWAP_PROGRAM_ID = new PublicKey('675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8');
export const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
export const TOKEN_2022_PROGRAM_ID = new PublicKey('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');
export const SYSTEM_PROGRAM_ID = new PublicKey('11111111111111111111111111111111');

// Token Mints
export const SOL_MINT = new PublicKey('So11111111111111111111111111111111111111112');
export const BONK_MINT = new PublicKey('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263');

// Dex enum values (matching the Rust enum)
export enum Dex {
  SplTokenSwap = 0,
  StableSwap = 1,
  Whirlpool = 2,
  MeteoraDynamicpool = 3,
  RaydiumSwap = 4,
  RaydiumStableSwap = 5,
  RaydiumClmmSwap = 6,
  AldrinExchangeV1 = 7,
  AldrinExchangeV2 = 8,
  LifinityV1 = 9,
  LifinityV2 = 10,
  RaydiumClmmSwapV2 = 11,
  FluxBeam = 12,
  MeteoraDlmm = 13,
  RaydiumCpmmSwap = 14,
  OpenBookV2 = 15,
  WhirlpoolV2 = 16,
  Phoenix = 17,
  ObricV2 = 18,
  SanctumAddLiq = 19,
  SanctumRemoveLiq = 20,
  SanctumNonWsolSwap = 21,
  SanctumWsolSwap = 22,
  PumpfunBuy = 23,
  PumpfunSell = 24,
  StabbleSwap = 25,
  SanctumRouter = 26,
  MeteoraVaultDeposit = 27,
  MeteoraVaultWithdraw = 28,
  Saros = 29,
  MeteoraLst = 30,
  Solfi = 31,
  QualiaSwap = 32,
  Zerofi = 33,
  PumpfunammBuy = 34,
  PumpfunammSell = 35,
  Virtuals = 36,
  VertigoBuy = 37,
  VertigoSell = 38,
  PerpetualsAddLiq = 39,
  PerpetualsRemoveLiq = 40,
  PerpetualsSwap = 41,
  RaydiumLaunchpad = 42,
  LetsBonkFun = 43,
  Woofi = 44,
  MeteoraDbc = 45,
  MeteoraDlmmSwap2 = 46,
  MeteoraDAMMV2 = 47,
  Gavel = 48,
  BoopfunBuy = 49,
  BoopfunSell = 50,
  MeteoraDbc2 = 51,
  GooseFX = 52,
  Dooar = 53,
  Numeraire = 54,
  SaberDecimalWrapperDeposit = 55,
  SaberDecimalWrapperWithdraw = 56,
  SarosDlmm = 57,
  OneDexSwap = 58,
  Manifest = 59,
  ByrealClmm = 60,
  PancakeSwapV3Swap = 61,
  PancakeSwapV3SwapV2 = 62,
  Tessera = 63,
  SolRfq = 64,
  PumpfunBuy2 = 65,
  PumpfunammBuy2 = 66,
}

// Route structure
export interface Route {
  dexes: Dex[];
  weights: number[];
}

// SwapArgs structure
export interface SwapArgs {
  amountIn: number;
  expectAmountOut: number;
  minReturn: number;
  amounts: number[];
  routes: Route[][];
}

// Account structure for instruction
export interface AccountMeta {
  pubkey: PublicKey;
  isSigner: boolean;
  isWritable: boolean;
}

// Raydium AMM pool info (you'll need to get this from Raydium API or SDK)
export interface RaydiumPoolInfo {
  ammId: PublicKey;
  ammAuthority: PublicKey;
  ammOpenOrders: PublicKey;
  ammTargetOrders?: PublicKey; // Deprecated - only used in legacy AMM v4
  poolCoinTokenAccount: PublicKey;
  poolPcTokenAccount: PublicKey;
  serumProgramId: PublicKey;
  serumMarket: PublicKey;
  serumBids: PublicKey;
  serumAsks: PublicKey;
  serumEventQueue: PublicKey;
  serumCoinVaultAccount: PublicKey;
  serumPcVaultAccount: PublicKey;
  serumVaultSigner: PublicKey;
}
