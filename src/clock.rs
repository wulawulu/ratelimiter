use std::{fmt::Debug, ops::Add, sync::{Arc, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};

use super::nanos::Nanos;

pub trait Reference:
    Sized + Add<Nanos, Output = Self> + PartialEq + Eq + Ord + Copy + Clone + Send + Sync + Debug
{
    fn duration_since(&self, earlier: Self) -> Nanos;
    fn saturating_sub(&self, duration: Nanos) -> Self;
}

pub trait Clock: Clone {
    type Instant: Reference;
    fn now(&self) -> Self::Instant;
}


#[derive(Clone,Debug,Default)]
pub struct MonotonicClock;

impl Reference for Instant {
    fn duration_since(&self, earlier: Self) -> Nanos {
        if earlier<*self{
            (*self - earlier).into()
        } else {
            Nanos::from(Duration::new(0, 0))
        }
    }

    fn saturating_sub(&self, duration: Nanos) -> Self {
        self.checked_sub(duration.into()).unwrap_or(*self)
    }
}

impl Add<Nanos> for Instant {
    type Output = Self;

    fn add(self, other: Nanos) -> Self::Output {
        let other: Duration = other.into();
        self + other
    }
}
    
impl Clock for MonotonicClock {
    type Instant = Instant;

    fn now(&self) -> Self::Instant {
        Instant::now()
    }
}
    

#[derive(Debug,Clone,Default)]
pub struct FakeRelativeClock{
    now:Arc<AtomicU64>,
}

impl FakeRelativeClock {
    pub fn advance(&self, by:Duration){
        let by:u64 = by
            .as_nanos()
            .try_into()
            .expect("Cannot represent duration greater than 584 years");

        let mut prev = self.now.load(Ordering::Acquire);
        let mut next = prev + by;
        while let Err(e)= self.now.compare_exchange_weak(prev, next, Ordering::Relaxed, Ordering::Relaxed){
            prev = e;
            next = prev + by;
        }
    }
}


impl Clock for FakeRelativeClock {
    type Instant = Nanos;

    fn now(&self) -> Self::Instant {
        self.now.load(Ordering::Acquire).into()
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_fake_relative_clock() {
        let clock = Arc::new(FakeRelativeClock::default());
        let threads = std::iter::repeat_n((), 10)
            .map(move |()|{
                let clock = clock.clone();
                thread::spawn(move||{
                    for _ in 0..1_000_000{
                        let now = clock.now();
                        clock.advance(Duration::from_nanos(1));
                        assert!(clock.now() > now);
                    }
                })
            })
            .collect::<Vec<_>>();

        for thread in threads {
            thread.join().unwrap();
        }
    }
}