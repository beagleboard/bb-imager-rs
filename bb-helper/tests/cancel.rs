#![cfg(feature = "cancel")]

use std::thread;

use bb_helper::cancel::CancellationToken;

#[test]
fn test_default_state() {
    let token = CancellationToken::default();
    assert!(
        !token.is_cancelled(),
        "Token should not be cancelled by default"
    );
}

#[test]
fn test_drop_guard_cancels() {
    let token = CancellationToken::default();

    // Create a guard in a nested scope and drop it
    {
        let _guard = token.drop_guard();
        assert!(
            !token.is_cancelled(),
            "Token should not be cancelled while guard is alive"
        );
    } // _guard is dropped here

    assert!(
        token.is_cancelled(),
        "Token should be cancelled after guard is dropped"
    );
}

#[test]
fn test_cloned_tokens_share_state() {
    let token1 = CancellationToken::default();
    let token2 = token1.clone();

    {
        let _guard = token1.drop_guard();
    }

    assert!(token1.is_cancelled());
    assert!(
        token2.is_cancelled(),
        "Cloned token should reflect the cancellation"
    );
}

#[test]
fn test_cross_thread_cancellation() {
    let token = CancellationToken::default();
    let token_clone = token.clone();

    let handle = thread::spawn(move || {
        assert!(!token_clone.is_cancelled());

        // Create and immediately drop the guard in the background thread
        let _guard = token_clone.drop_guard();
    });

    handle.join().unwrap();

    assert!(
        token.is_cancelled(),
        "Main thread should see cancellation from background thread"
    );
}
