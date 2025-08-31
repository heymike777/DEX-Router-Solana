import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';
import { SwapArgs, Route, Dex, OKX_DEX_ROUTER_PROGRAM_ID, AccountMeta } from './types';

// Instruction discriminator for swap instruction
const SWAP_INSTRUCTION_DISCRIMINATOR = Buffer.from([248, 198, 158, 145, 225, 117, 135, 200]);

/**
 * Serialize SwapArgs to buffer for instruction data
 */
export function serializeSwapArgs(args: SwapArgs): Buffer {
  const buffer = Buffer.alloc(1024); // Allocate enough space
  let offset = 0;

  // Write instruction discriminator
  SWAP_INSTRUCTION_DISCRIMINATOR.copy(buffer, offset);
  offset += 8;

  // Write amount_in (u64)
  buffer.writeBigUInt64LE(BigInt(args.amountIn), offset);
  offset += 8;

  // Write expect_amount_out (u64)
  buffer.writeBigUInt64LE(BigInt(args.expectAmountOut), offset);
  offset += 8;

  // Write min_return (u64)
  buffer.writeBigUInt64LE(BigInt(args.minReturn), offset);
  offset += 8;

  // Write amounts vector length (u32)
  buffer.writeUInt32LE(args.amounts.length, offset);
  offset += 4;

  // Write amounts (u64 array)
  for (const amount of args.amounts) {
    buffer.writeBigUInt64LE(BigInt(amount), offset);
    offset += 8;
  }

  // Write routes vector length (u32)
  buffer.writeUInt32LE(args.routes.length, offset);
  offset += 4;

  // Write routes
  for (const routeGroup of args.routes) {
    // Write route group length (u32)
    buffer.writeUInt32LE(routeGroup.length, offset);
    offset += 4;

    for (const route of routeGroup) {
      // Write dexes vector length (u32)
      buffer.writeUInt32LE(route.dexes.length, offset);
      offset += 4;

      // Write dexes (u8 array)
      for (const dex of route.dexes) {
        buffer.writeUInt8(dex, offset);
        offset += 1;
      }

      // Write weights vector length (u32)
      buffer.writeUInt32LE(route.weights.length, offset);
      offset += 4;

      // Write weights (u8 array)
      for (const weight of route.weights) {
        buffer.writeUInt8(weight, offset);
        offset += 1;
      }
    }
  }

  // Write order_id (u64) - using timestamp as order ID
  const orderId = BigInt(Date.now());
  buffer.writeBigUInt64LE(orderId, offset);
  offset += 8;

  return buffer.slice(0, offset);
}

/**
 * Get or create associated token account address
 */
export async function getOrCreateAssociatedTokenAccount(
  mint: PublicKey,
  owner: PublicKey,
  payer: PublicKey
): Promise<PublicKey> {
  return await getAssociatedTokenAddress(mint, owner, false);
}

/**
 * Create swap instruction for OKX DEX Router
 */
export function createSwapInstruction(
    payer: PublicKey,
    sourceTokenAccount: PublicKey,
    destinationTokenAccount: PublicKey,
    sourceMint: PublicKey,
    destinationMint: PublicKey,
    swapArgs: SwapArgs
): TransactionInstruction {
    const data = serializeSwapArgs(swapArgs);
    
    const keys: AccountMeta[] = [
        { pubkey: payer, isSigner: true, isWritable: false },
        { pubkey: sourceTokenAccount, isSigner: false, isWritable: true },
        { pubkey: destinationTokenAccount, isSigner: false, isWritable: true },
        { pubkey: sourceMint, isSigner: false, isWritable: false },
        { pubkey: destinationMint, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: OKX_DEX_ROUTER_PROGRAM_ID,
        data,
    });
}

/**
 * Convert SOL amount to lamports
 */
export function solToLamports(sol: number): number {
  return Math.floor(sol * 1e9);
}

/**
 * Convert lamports to SOL
 */
export function lamportsToSol(lamports: number): number {
  return lamports / 1e9;
}

/**
 * Calculate minimum return amount with slippage tolerance
 */
export function calculateMinReturn(amountOut: number, slippageTolerance: number): number {
  return Math.floor(amountOut * (1 - slippageTolerance));
}

/**
 * Create a simple swap route for Raydium
 */
export function createRaydiumSwapRoute(): Route[][] {
  return [[
    {
      dexes: [Dex.RaydiumSwap],
      weights: [100] // 100% weight for Raydium
    }
  ]];
}

/**
 * Create a multi-hop swap route for Raydium: WSOL → BONK → WSOL
 */
export function createMultiHopRaydiumSwapRoute(): Route[][] {
  return [
    // First hop: WSOL → BONK
    [{
      dexes: [Dex.RaydiumSwap],
      weights: [100] // 100% weight for Raydium
    }],
    // Second hop: BONK → WSOL
    [{
      dexes: [Dex.RaydiumSwap],
      weights: [100] // 100% weight for Raydium
    }]
  ];
}
