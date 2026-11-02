use std::sync::{Arc, Mutex, RwLock};

use super::{lock, read, write};

/// Panics inside a thread while it holds the mutex, which poisons it.
fn poison_mutex(mutex: &Arc<Mutex<u32>>) {
    let shared = Arc::clone(mutex);
    let outcome = std::thread::spawn(move || {
        let _guard = shared.lock().unwrap();
        panic!("poisoning the lock on purpose");
    })
    .join();
    assert!(outcome.is_err());
    assert!(mutex.is_poisoned());
}

fn poison_rwlock(rwlock: &Arc<RwLock<u32>>) {
    let shared = Arc::clone(rwlock);
    let outcome = std::thread::spawn(move || {
        let _guard = shared.write().unwrap();
        panic!("poisoning the lock on purpose");
    })
    .join();
    assert!(outcome.is_err());
    assert!(rwlock.is_poisoned());
}

/// REQ-SEC-014: a mutex poisoned by a panic stays readable and writable
/// through `lock`.
#[test]
fn req_sec_014_lock_recovers_a_poisoned_mutex() {
    let mutex = Arc::new(Mutex::new(7));
    poison_mutex(&mutex);
    assert_eq!(*lock(&mutex), 7);
    *lock(&mutex) += 1;
    assert_eq!(*lock(&mutex), 8);
}

/// REQ-SEC-014: a read-write lock poisoned by a panic stays readable
/// through `read`.
#[test]
fn req_sec_014_read_recovers_a_poisoned_rwlock() {
    let rwlock = Arc::new(RwLock::new(7));
    poison_rwlock(&rwlock);
    assert_eq!(*read(&rwlock), 7);
    // Two readers at once still work.
    let first = read(&rwlock);
    let second = read(&rwlock);
    assert_eq!(*first + *second, 14);
}

/// REQ-SEC-014: a read-write lock poisoned by a panic stays writable
/// through `write`.
#[test]
fn req_sec_014_write_recovers_a_poisoned_rwlock() {
    let rwlock = Arc::new(RwLock::new(7));
    poison_rwlock(&rwlock);
    *write(&rwlock) += 1;
    assert_eq!(*read(&rwlock), 8);
}

/// REQ-SEC-014: the helpers behave as plain locks when nothing is poisoned.
#[test]
fn req_sec_014_helpers_work_on_healthy_locks() {
    let mutex = Mutex::new(1);
    *lock(&mutex) = 2;
    assert_eq!(*lock(&mutex), 2);
    let rwlock = RwLock::new(1);
    *write(&rwlock) = 3;
    assert_eq!(*read(&rwlock), 3);
}
