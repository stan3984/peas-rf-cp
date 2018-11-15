use rand::RngCore;

/// 64-bit unsigned integer used a unique identifier.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Id(u64);

impl Id {
    /// Creates a new `Id` with value `x`.
    #[inline]
    pub fn from_u64(x: u64) -> Id {
        Id(x)
    }

    /// Creates a new `Id` whose value is random.
    #[inline]
    pub fn new_random() -> Id {
        let mut rng = rand::thread_rng();
        let id = rng.next_u64();
        Id(id)
    }

    /// Computes the "distance" between this `Id` and another one.
    #[inline]
    pub fn distance(&self, other: &Id) -> u64 {
        self.0 ^ other.0
    }

    /// Consumes this `Id` and returns its value.
    #[inline]
    pub fn into_u64(self) -> u64 {
        self.0
    }

    /// returns the number of leading common/equal bits between this
    /// `Id` and another one.
    #[inline]
    pub fn common_bits(&self, other: &Id) -> u32 {
        (self.0 ^ other.0).leading_zeros()
    }

}
