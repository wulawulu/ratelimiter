use std::{
    fmt::Debug,
    ops::{Add, Div, Mul},
    time::Duration,
};

use crate::clock;

#[derive(PartialEq, Eq, Default, Clone, Copy, PartialOrd, Ord)]
pub struct Nanos(u64);

impl Nanos {
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Nanos {
    pub const fn new(u: u64) -> Self {
        Self(u)
    }
}

impl From<Duration> for Nanos {
    fn from(duration: Duration) -> Self {
        Self(
            duration
                .as_nanos()
                .try_into()
                .expect("Duration is longer than 584 years"),
        )
    }
}

impl Debug for Nanos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let d = Duration::from_nanos(self.0);
        write!(f, "Nanos({d:?})")
    }
}

impl Add<Self> for Nanos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<u64> for Nanos {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<Self> for Nanos {
    type Output = u64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl From<u64> for Nanos {
    fn from(u: u64) -> Self {
        Self(u)
    }
}

impl From<Nanos> for u64 {
    fn from(n: Nanos) -> Self {
        n.0
    }
}

impl From<Nanos> for Duration {
    fn from(n: Nanos) -> Self {
        Self::from_nanos(n.0)
    }
}

impl Nanos {
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Add<Duration> for Nanos {
    type Output = Self;

    fn add(self, other: Duration) -> Self::Output {
        let other: Self = other.into();
        self + other
    }
}

impl clock::Reference for Nanos {
    #[inline]
    fn duration_since(&self, earlier: Self) -> Nanos {
        (*self as Self).saturating_sub(earlier)
    }

    #[inline]
    fn saturating_sub(&self, duration: Nanos) -> Self {
        (*self as Self).saturating_sub(duration)
    }
}
