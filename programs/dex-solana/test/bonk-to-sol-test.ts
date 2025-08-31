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
} from '@solana/spl-token';
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

// Test parameters - we'll swap a small amount of BONK back to SOL
const BONK_AMOUNT = 10000 * (10 ** 5); // 10,000 BONK (about 0.001 SOL worth)

async function main() {
    console.log('🚀 Starting OKX DEX Router BONK to SOL Swap Test');
    console.log('================================================');
    console.log('Route: BONK → WSOL');

    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');
    console.log(`📡 Connected to: ${RPC_URL}`);

    // Load wallet
    if (!PRIVATE_KEY) {
        throw new Error('Please set PRIVATE_KEY in your .env file');
    }

    const wallet = Keypair.fromSecretKey(Buffer.from(JSON.parse(PRIVATE_KEY)));
    console.log(`👛 Wallet: ${wallet.publicKey.toString()}`);

    // Get token accounts
    console.log('\n🔍 Setting up token accounts...');
    
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
        console.log(`🔄 wSOL balance: ${wsolAccountInfo.amount.toLocaleString()}`);
    } catch (error) {
        console.log(`❌ wSOL token account not found`);
        throw new Error('wSOL token account not found. Please run the SOL to BONK swap first.');
    }
    
    const bonkTokenAccount = await getAssociatedTokenAddress(
        BONK_MINT,
        wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
    );

    // Check if BONK token account exists and has balance
    let bonkAccountInfo;
    try {
        bonkAccountInfo = await getAccount(connection, bonkTokenAccount);
        console.log(`✅ BONK token account exists: ${bonkTokenAccount.toString()}`);
        console.log(`🐕 BONK balance: ${bonkAccountInfo.amount.toLocaleString()}`);
        
        if (bonkAccountInfo.amount < BONK_AMOUNT) {
            throw new Error(`Insufficient BONK balance. Have: ${bonkAccountInfo.amount.toLocaleString()}, Need: ${BONK_AMOUNT.toLocaleString()}`);
        }
    } catch (error) {
        console.log(`❌ BONK token account not found or insufficient balance`);
        throw new Error('BONK token account not found or insufficient balance. Please run the SOL to BONK swap first.');
    }

    // Fetch Raydium pool information
    console.log('\n🏊 Fetching Raydium pool information...');
    const poolInfo = SOL_BONK_POOL_INFO;
    console.log(`✅ Pool found: ${poolInfo.ammId.toString()}`);

    console.log(`💱 Swap amount: ${BONK_AMOUNT.toLocaleString()} BONK`);

    // Create transaction
    console.log('\n🔨 Building transaction...');
    const transaction = new Transaction();

    // Create the swap instruction: BONK → WSOL
    console.log('💱 Adding swap instruction (BONK → WSOL)...');
    const swapArgs: SwapArgs = {
        amountIn: BONK_AMOUNT,
        expectAmountOut: 1, // We'll set this to 1 for now
        minReturn: 1, // We'll set this to 1 for now
        amounts: [BONK_AMOUNT],
        routes: createRaydiumSwapRoute(),
    };

    const swapInstruction = createSwapInstruction(
        wallet.publicKey,
        bonkTokenAccount, // Source: BONK
        wsolTokenAccount, // Destination: wSOL
        BONK_MINT,
        SOL_MINT,
        swapArgs
    );

    // Add all required Raydium accounts to the instruction
    const requiredAccounts = [
        RAYDIUM_SWAP_PROGRAM_ID, // dex_program_id
        wallet.publicKey, // swap_authority_pubkey - should be the user's wallet
        bonkTokenAccount, // swap_source_token (BONK)
        wsolTokenAccount, // swap_destination_token (WSOL)
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
        console.log('1. Make sure you have enough BONK for the swap');
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
