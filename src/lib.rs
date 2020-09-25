#![allow(non_snake_case)]
#![allow(dead_code)]
#![feature(test)]
extern crate test;

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering, spin_loop_hint};
use std::thread::yield_now;

pub struct Putex<T> {
    data: UnsafeCell<T>,
    lock: AtomicUsize,
    spin: bool,
}

unsafe impl<T> Sync for Putex<T> {}

pub struct PutexGuard<'a, T> {
    data: &'a mut T,
    putex: &'a Putex<T>,
}

impl<T> Putex<T> {
    pub fn new(data: T, spin: bool) -> Self {
        Self {
            data: UnsafeCell::new(data),
            lock: AtomicUsize::new(0),
            spin,
        }
    }

    pub fn lock(&self) -> PutexGuard<T> {
        let awaiter = if self.spin { spin_loop_hint } else { yield_now };
        loop {
            let prev = self.lock.compare_and_swap(0, 1, Ordering::AcqRel);
            if prev == 0 {
                // lock successful
                break;
            }
            awaiter();
        }

        PutexGuard {
            data: unsafe { self.data.get().as_mut().unwrap() },
            putex: self,
        }
    }

    fn unlock(&self) {
        let prev = self.lock.compare_and_swap(1, 0, Ordering::AcqRel);
        if prev != 1 {
            panic!(
                "Called unlock() on Putex with lock value != 1 (was: {})",
                prev
            );
        }
    }
}

impl<'a, T> Drop for PutexGuard<'a, T> {
    fn drop(&mut self) {
        self.putex.unlock();
    }
}

impl<'a, T> Deref for PutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T> DerefMut for PutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    struct UnsafeCellSync(UnsafeCell<i32>);
    unsafe impl Sync for UnsafeCellSync {}

    /// Sanity check, no synchronization at all should fail
    fn no_mutex() {
        use std::sync::Arc;
        use std::thread;

        let x = Arc::new(UnsafeCellSync {
            0: UnsafeCell::new(0),
        });
        let x2 = x.clone();
        let x3 = x.clone();

        let t1 = thread::spawn(move || {
            for i in 0..10000 {
                unsafe {
                    *x.0.get() += 1;
                }
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        let t2 = thread::spawn(move || {
            for i in 0..10000 {
                unsafe {
                    *x2.0.get() -= 1;
                }
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        t1.join().expect("t1 join failed");
        t2.join().expect("t2 join failed");

        let res = unsafe { *x3.0.get() };
        assert_eq!(res, 0);
    }

    // doesn't work, so can't benchmark - only test
    #[test]
    #[should_panic]
    fn test_no_mutex() {
        for _ in 0..100 {
            no_mutex();
        }
    }

    /// Regular std::sync::Mutex
    fn std_mutex() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let mtx = Arc::new(Mutex::new(0));
        let mtx2 = mtx.clone();
        let mtx3 = mtx.clone();

        let t1 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx.lock().unwrap();
                *inner += 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        let t2 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx2.lock().unwrap();
                *inner -= 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        t1.join().expect("t1 join failed");
        t2.join().expect("t2 join failed");

        let res = *mtx3.lock().unwrap();
        assert_eq!(res, 0);
    }

    #[bench]
    fn bench_std_mutex(b: &mut Bencher) {
        b.iter(|| std_mutex());
    }

    /// Custom spinlock based Mutex
    fn putex_spin() {
        use std::sync::Arc;
        use std::thread;

        let mtx = Arc::new(Putex::new(0, true));
        let mtx2 = mtx.clone();
        let mtx3 = mtx.clone();

        let t1 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx.lock();
                *inner += 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        let t2 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx2.lock();
                *inner -= 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        t1.join().expect("t1 join failed");
        t2.join().expect("t2 join failed");

        let res = *mtx3.lock();
        assert_eq!(res, 0);
    }

    #[bench]
    fn bench_putex_spin(b: &mut Bencher) {
        b.iter(|| putex_spin());
    }

    /// Custom OS scheduler yield based Mutex
    fn putex_yield() {
        use std::sync::Arc;
        use std::thread;

        let mtx = Arc::new(Putex::new(0, false));
        let mtx2 = mtx.clone();
        let mtx3 = mtx.clone();

        let t1 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx.lock();
                *inner += 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        let t2 = thread::spawn(move || {
            for i in 0..10000 {
                let mut inner = mtx2.lock();
                *inner -= 1;
                if i % 100 == 0 {
                    yield_now();
                }
            }
        });

        t1.join().expect("t1 join failed");
        t2.join().expect("t2 join failed");

        let res = *mtx3.lock();
        assert_eq!(res, 0);
    }

    #[bench]
    fn bench_putex_yield(b: &mut Bencher) {
        b.iter(|| putex_yield());
    }
}
