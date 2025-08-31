import { PublicKey } from '@solana/web3.js';
import { RaydiumPoolInfo, SOL_MINT, BONK_MINT } from './types';

// This is a sample pool info for SOL-BONK on Raydium
// In a real application, you would fetch this from Raydium's API or SDK
export const SOL_BONK_POOL_INFO_1: RaydiumPoolInfo = {
  ammId: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  ammAuthority: new PublicKey('5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1'),
  ammOpenOrders: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  ammTargetOrders: undefined,
  poolCoinTokenAccount: new PublicKey('7KFdXKA5WkZBspxwqd9kSrDGTg9WhiX5TptUB3yRwEaE'),
  poolPcTokenAccount: new PublicKey('GehmCo7EgzkB4xxyviW6xdUhm1Ed2nN98QcfcRWQCfA9'),
  serumProgramId: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumMarket: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumBids: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumAsks: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumEventQueue: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumCoinVaultAccount: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumPcVaultAccount: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
  serumVaultSigner: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
};

// This one is with OPENBOOK.
// tx: 5tYuPAXTeyyhAjGUfWMMCYrMeDSyorGVk7sGof2SsjiVK6J3UK1xxwQswJBx4E4nmTx9WJVsDPFo6w6ab3wGc3YG
export const SOL_BONK_POOL_INFO_2: RaydiumPoolInfo = {
    ammId: new PublicKey('HVNwzt7Pxfu76KHCMQPTLuTCLTm6WnQ1esLv4eizseSv'),
    ammAuthority: new PublicKey('5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1'),
    ammOpenOrders: new PublicKey('5s1WVFfMEuBG3vRG5UWjxZRrt4b5bQJ9FicUbpTJQwnh'),
    ammTargetOrders: new PublicKey('62h3u5gz9SmafX6d5v19ixziM15vHmHzZojuEQoLdXsT'),
    poolCoinTokenAccount: new PublicKey('7KFdXKA5WkZBspxwqd9kSrDGTg9WhiX5TptUB3yRwEaE'),
    poolPcTokenAccount: new PublicKey('GehmCo7EgzkB4xxyviW6xdUhm1Ed2nN98QcfcRWQCfA9'),
    serumProgramId: new PublicKey('srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX'),
    serumMarket: new PublicKey('Hs97TCZeuYiJxooo3U73qEHXg3dKpRL4uYKYRryEK9CF'),
    serumBids: new PublicKey('FyyGTHKJBf1nGHHL44HE91EcGRFg2Y7XazA3SjQpcU3i'),
    serumAsks: new PublicKey('2KajVpMkF3Z53MgTgo7dDh23av6xWKgKssbtjogdY7Vu'),
    serumEventQueue: new PublicKey('BBRvF4etMRitpSwFdXSMzPg547Lxnr3G95aQtBhMiWhB'),
    serumCoinVaultAccount: new PublicKey('AVnL1McPPrn1dZyHGThwXzwYaHBp6sxB44vXoETPqH45'),
    serumPcVaultAccount: new PublicKey('8KftabityJoWgvUb6wwAPZww8mYmLq8WTMuQGGPoGiKM'),
    serumVaultSigner: new PublicKey('51Cdt3oASXuVD88tAqJEeR6XH3PjQQ3xb7Cd22KaW2GK'),
};

export const SOL_BONK_POOL_INFO = SOL_BONK_POOL_INFO_2;


/**
 * Fetch Raydium pool information for a given token pair
 * In a real application, you would use Raydium's API or SDK to get this data
 */
export async function fetchRaydiumPoolInfo(
  tokenAMint: PublicKey,
  tokenBMint: PublicKey
): Promise<RaydiumPoolInfo> {
  // For this example, we'll return the hardcoded SOL-BONK pool
  // In production, you would:
  // 1. Call Raydium's API to get pool information
  // 2. Or use Raydium's SDK to fetch pool data
  // 3. Or query on-chain data to find the pool
  
  if (tokenAMint.equals(SOL_MINT) && tokenBMint.equals(BONK_MINT)) {
    return SOL_BONK_POOL_INFO;
  }
  
  throw new Error(`Pool not found for ${tokenAMint.toString()} - ${tokenBMint.toString()}`);
}

/**
 * Get all required accounts for a Raydium swap
 */
export function getRaydiumSwapAccounts(poolInfo: RaydiumPoolInfo) {
  return [
    poolInfo.ammId,
    poolInfo.ammAuthority,
    poolInfo.ammOpenOrders,
    poolInfo.ammTargetOrders,
    poolInfo.poolCoinTokenAccount,
    poolInfo.poolPcTokenAccount,
    poolInfo.serumProgramId,
    poolInfo.serumMarket,
    poolInfo.serumBids,
    poolInfo.serumAsks,
    poolInfo.serumEventQueue,
    poolInfo.serumCoinVaultAccount,
    poolInfo.serumPcVaultAccount,
    poolInfo.serumVaultSigner,
  ];
}
