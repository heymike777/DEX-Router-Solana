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
const SOL_AMOUNT = 0.01; // 0.01 SOL

async function main() {
    console.log('🚀 Starting OKX DEX Router BONK Swap Test');
    console.log('==========================================');

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

    // Fetch Raydium pool information and get price quote
    console.log('\n🏊 Fetching Raydium pool information...');
    const inputAmount = solToLamports(SOL_AMOUNT);
    const poolInfo = SOL_BONK_POOL_INFO;
    console.log(`✅ Pool found: ${poolInfo.ammId.toString()}`);

    const swapArgs: SwapArgs = {
        amountIn: inputAmount,
        expectAmountOut: 1,//TODO: how can I set it to ZERO?
        minReturn: 1,//TODO: how can I set it to ZERO?
        amounts: [inputAmount], // Single amount for single route
        routes: createRaydiumSwapRoute(),
    };

    console.log(`💱 Swap amount: ${SOL_AMOUNT} SOL`);

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

    // Create the swap instruction
    console.log('💱 Adding swap instruction...');
    const swapInstruction = createSwapInstruction(
        wallet.publicKey,
        wsolTokenAccount, // Use wSOL token account as source
        bonkTokenAccount,
        SOL_MINT,
        BONK_MINT,
        swapArgs
    );

    // Add all required Raydium accounts to the instruction in the correct order
    // The order should match RaydiumSwapAccounts structure
    const requiredAccounts = [
        RAYDIUM_SWAP_PROGRAM_ID, // dex_program_id
        wallet.publicKey, // swap_authority_pubkey - should be the user's wallet (owner of source token account)
        wsolTokenAccount, // swap_source_token (will be replaced by the program)
        bonkTokenAccount, // swap_destination_token (will be replaced by the program)
        TOKEN_PROGRAM_ID, // token_program
        poolInfo.ammId, // amm_id
        poolInfo.ammAuthority, // amm_authority
        poolInfo.ammOpenOrders, // amm_open_orders
        poolInfo.ammTargetOrders, // amm_target_orders
        poolInfo.poolCoinTokenAccount, // pool_coin_token_account
        poolInfo.poolPcTokenAccount, // pool_pc_token_account
        poolInfo.serumProgramId, // serum_program_id
        poolInfo.serumMarket, // serum_market
        poolInfo.serumBids, // serum_bids
        poolInfo.serumAsks, // serum_asks
        poolInfo.serumEventQueue, // serum_event_queue
        poolInfo.serumCoinVaultAccount, // serum_coin_vault_account
        poolInfo.serumPcVaultAccount, // serum_pc_vault_account
        poolInfo.serumVaultSigner, // serum_vault_signer
    ];

    requiredAccounts.forEach((account, index) => {
        if (account){
            // The swap authority (user's wallet) should be a signer
            const isSigner = index === 1; // swap_authority_pubkey is at index 1
            swapInstruction.keys.push({
                pubkey: account,
                isSigner: isSigner,
                isWritable: true,
            });
        }
    });

    transaction.add(swapInstruction);

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
            const finalBonkAccount = await getAccount(connection, bonkTokenAccount);
            console.log(`🐕 BONK: ${finalBonkAccount.amount.toLocaleString()}`);
        } catch (error) {
            console.log(`🐕 BONK: Account not found (swap may have failed)`);
        }

    } catch (error) {
        console.error('❌ Transaction failed:', error);
        
        // Log detailed error information
        if (error instanceof Error) {
            console.error('Error message:', error.message);
        }
        
        // You might want to check the transaction logs for more details
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
