//! What the tests share.

use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

static HEADLESS: Mutex<()> = Mutex::new(());

thread_local! {
    static HELD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
}

/// Keeps the other tests that run a headless app from running one while this
/// test does, until the test is over.
///
/// Headless apps run alongside each other take each other's taps and swipes and
/// miss their frames, which showed as tests that failed now and then. Called
/// before the app is started; calling it again in the same test changes nothing.
pub fn hold() {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        if held.is_none() {
            *held = Some(HEADLESS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
        }
    });
}
