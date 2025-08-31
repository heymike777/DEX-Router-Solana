import {
  Connection,
  Keypair,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import {
  getAssociatedTokenAddress,
  getAccount,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createSyncNativeInstruction,
} from '@solana/spl-token';
import { SystemProgram } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import {
  OKX_DEX_ROUTER_PROGRAM_ID,
  RAYDIUM_SWAP_PROGRAM_ID,
  SOL_MINT,
  BONK_MINT,
  SwapArgs,
} from '../src/types';
import {
  createSwapInstruction,
  solToLamports,
  createRaydiumSwapRoute,
} from '../src/utils';
import { SOL_BONK_POOL_INFO } from '../src/raydium-pool';

dotenv.config();

// Configuration
const RPC_URL = process.env.RPC_URL || 'https://api.mainnet-beta.solana.com';
const PRIVATE_KEY = process.env.PRIVATE_KEY || '';

// Test parameters
const SOL_AMOUNT = 0.005; // 0.005 SOL

async function main() {
    console.log('🚀 Starting OKX DEX Router Two Swaps Test');
    console.log('=========================================');
    console.log('Route: WSOL → BONK → WSOL (two separate swaps)');

    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');
    console.log(`📡 Connected to: ${RPC_URL}`);

    // Load wallet
    if (!PRIVATE_KEY) {
        throw new Error('Please set PRIVATE_KEY in your .env file');
    }

    const wallet = Keypair.fromSecretKey(Buffer.from(JSON.parse(PRIVATE_KEY)));
    console.log(`👛 Wallet: ${wallet.publicKey.toString()}`);

    // Check wallet balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`💰 Wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);

    if (balance < solToLamports(SOL_AMOUNT + 0.01)) {
        throw new Error('Insufficient SOL balance for swap and fees');
    }

    // Get or create token accounts
    console.log('\n🔍 Setting up token accounts...');
    
    // Get wSOL token account address (wSOL mint is the same as SOL mint)
    const wsolTokenAccount = await getAssociatedTokenAddress(
        SOL_MINT,
        wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
    );

    // Check if wSOL token account exists
    let wsolAccountInfo;
    try {
        wsolAccountInfo = await getAccount(connection, wsolTokenAccount);
        console.log(`✅ wSOL token account exists: ${wsolTokenAccount.toString()}`);
    } catch (error) {
        console.log(`❌ wSOL token account not found, will create it`);
    }
    
    const bonkTokenAccount = await getAssociatedTokenAddress(
        BONK_MINT,
        wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
    );

    // Check if BONK token account exists
    let bonkAccountInfo;
    try {
        bonkAccountInfo = await getAccount(connection, bonkTokenAccount);
        console.log(`✅ BONK token account exists: ${bonkTokenAccount.toString()}`);
    } catch (error) {
        console.log(`❌ BONK token account not found, will create it`);
    }

    // Fetch Raydium pool information
    console.log('\n🏊 Fetching Raydium pool information...');
    const inputAmount = solToLamports(SOL_AMOUNT);
    const poolInfo = SOL_BONK_POOL_INFO;
    console.log(`✅ Pool found: ${poolInfo.ammId.toString()}`);

    console.log(`💱 Swap amount: ${SOL_AMOUNT} SOL`);
    console.log(`🔄 Route: WSOL → BONK → WSOL (two separate swaps)`);

    // Create transaction
    console.log('\n🔨 Building transaction...');
    const transaction = new Transaction();

    // Add instruction to create wSOL token account if it doesn't exist
    if (!wsolAccountInfo) {
        console.log('➕ Adding create wSOL token account instruction...');
        const createWsolAtaIx = createAssociatedTokenAccountIdempotentInstruction(
            wallet.publicKey,
            wsolTokenAccount,
            wallet.publicKey,
            SOL_MINT,
            TOKEN_PROGRAM_ID
        );
        transaction.add(createWsolAtaIx);
    }

    // Add instruction to create BONK token account if it doesn't exist
    if (!bonkAccountInfo) {
        console.log('➕ Adding create BONK token account instruction...');
        const createBonkAtaIx = createAssociatedTokenAccountIdempotentInstruction(
            wallet.publicKey,
            bonkTokenAccount,
            wallet.publicKey,
            BONK_MINT,
            TOKEN_PROGRAM_ID
        );
        transaction.add(createBonkAtaIx);
    }

    // Add instruction to wrap SOL to wSOL
    console.log('🔄 Adding SOL wrap instruction...');
    const wrapSolIx = createSyncNativeInstruction(wsolTokenAccount);
    transaction.add(wrapSolIx);

    // Add instruction to transfer SOL to wSOL account for wrapping
    console.log('💰 Adding SOL transfer instruction...');
    const transferSolIx = SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: wsolTokenAccount,
        lamports: inputAmount,
    });
    transaction.add(transferSolIx);

    // First swap: WSOL → BONK
    console.log('💱 Adding first swap instruction (WSOL → BONK)...');
    const firstSwapArgs: SwapArgs = {
        amountIn: inputAmount,
        expectAmountOut: 1,
        minReturn: 1,
        amounts: [inputAmount],
        routes: createRaydiumSwapRoute(),
    };

    const firstSwapInstruction = createSwapInstruction(
        wallet.publicKey,
        wsolTokenAccount, // Source: wSOL
        bonkTokenAccount, // Destination: BONK
        SOL_MINT,
        BONK_MINT,
        firstSwapArgs
    );

    // Add Raydium accounts for first swap
    const firstSwapAccounts = [
        RAYDIUM_SWAP_PROGRAM_ID,
        wallet.publicKey, // swap_authority_pubkey
        wsolTokenAccount, // swap_source_token (WSOL)
        bonkTokenAccount, // swap_destination_token (BONK)
        TOKEN_PROGRAM_ID,
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

    firstSwapAccounts.forEach((account, index) => {
        if (account){
            const isSigner = index === 1; // swap_authority_pubkey
            firstSwapInstruction.keys.push({
                pubkey: account,
                isSigner: isSigner,
                isWritable: true,
            });
        }
    });

    transaction.add(firstSwapInstruction);

    // Second swap: BONK → WSOL (we'll use the BONK balance we just received)
    console.log('💱 Adding second swap instruction (BONK → WSOL)...');
    const secondSwapArgs: SwapArgs = {
        amountIn: 1, // We'll set this to 1 for now, the actual amount will be the BONK balance
        expectAmountOut: 1,
        minReturn: 1,
        amounts: [1], // We'll set this to 1 for now
        routes: createRaydiumSwapRoute(),
    };

    const secondSwapInstruction = createSwapInstruction(
        wallet.publicKey,
        bonkTokenAccount, // Source: BONK
        wsolTokenAccount, // Destination: wSOL
        BONK_MINT,
        SOL_MINT,
        secondSwapArgs
    );

    // Add Raydium accounts for second swap (reverse direction)
    const secondSwapAccounts = [
        RAYDIUM_SWAP_PROGRAM_ID,
        wallet.publicKey, // swap_authority_pubkey
        bonkTokenAccount, // swap_source_token (BONK)
        wsolTokenAccount, // swap_destination_token (WSOL)
        TOKEN_PROGRAM_ID,
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

    secondSwapAccounts.forEach((account, index) => {
        if (account){
            const isSigner = index === 1; // swap_authority_pubkey
            secondSwapInstruction.keys.push({
                pubkey: account,
                isSigner: isSigner,
                isWritable: true,
            });
        }
    });

    transaction.add(secondSwapInstruction);

    // Get recent blockhash
    console.log('📡 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = wallet.publicKey;

    // Sign and send transaction
    console.log('\n🚀 Sending transaction...');
    try {
        const signature = await sendAndConfirmTransaction(
            connection,
            transaction,
            [wallet],
            {
                commitment: 'confirmed',
                preflightCommitment: 'confirmed',
            }
        );

        console.log(`✅ Transaction successful!`);
        console.log(`🔗 Signature: ${signature}`);
        console.log(`🔍 View on Solscan: https://solscan.io/tx/${signature}`);

        // Check final balances
        console.log('\n📊 Final balances:');
        const finalSolBalance = await connection.getBalance(wallet.publicKey);
        console.log(`💰 SOL: ${finalSolBalance / LAMPORTS_PER_SOL}`);

        try {
            const finalWsolAccount = await getAccount(connection, wsolTokenAccount);
            console.log(`🔄 wSOL: ${finalWsolAccount.amount.toLocaleString()}`);
        } catch (error) {
            console.log(`🔄 wSOL: Account not found`);
        }

        try {
            const finalBonkAccount = await getAccount(connection, bonkTokenAccount);
            console.log(`🐕 BONK: ${finalBonkAccount.amount.toLocaleString()}`);
        } catch (error) {
            console.log(`🐕 BONK: Account not found`);
        }

    } catch (error) {
        console.error('❌ Transaction failed:', error);
        
        // Log detailed error information
        if (error instanceof Error) {
            console.error('Error message:', error.message);
        }
        
        console.log('\n💡 Tips for debugging:');
        console.log('1. Make sure you have enough SOL for the swap and fees');
        console.log('2. Check that the Raydium pool information is correct');
        console.log('3. Verify that the expected amounts are reasonable');
        console.log('4. Check the transaction logs on Solscan for detailed error information');
    }
}

// Error handling for unhandled promise rejections
process.on('unhandledRejection', (error) => {
    console.error('Unhandled promise rejection:', error);
    process.exit(1);
});

// Run the test
if (require.main === module) {
    main()
        .then(() => {
            console.log('\n🎉 Test completed!');
            process.exit(0);
        })
        .catch((error) => {
            console.error('\n💥 Test failed:', error);
            process.exit(1);
        });
}
