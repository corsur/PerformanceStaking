use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use anchor_client::solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn test_initialize_and_stake_allocation() {
    let program_id = Pubkey::new_unique();
    let mut test_env = ProgramTest::new("five90", program_id, processor!(five90::entry));

    let (mut banks_client, payer, recent_blockhash) = test_env.start().await;

    // Initialize the program
    let init_tx = Transaction::new_signed_with_payer(
        &[initialize_instruction()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(init_tx).await.unwrap();

    // Update stake allocations
    let update_tx = Transaction::new_signed_with_payer(
        &[update_stake_allocations_instruction(5)],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(update_tx).await.unwrap();
}