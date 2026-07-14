use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
};

/// A floating point number, with custom equality, ordering and hashing.
/// All NaNs are considered to be equal, regardless of sign.  -0.0 and +0.0 are
/// considered to be different.  Otherwise, numbers are handled using [`f64::total_order`]
#[derive(Debug, Clone, Copy)]
pub struct OrderedFloat(pub f64);

impl Deref for OrderedFloat {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OrderedFloat {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        // all NaNs are equal (and positive), -0.0 != +0.0
        if self.is_nan() && other.is_nan() {
            true
        } else {
            self.total_cmp(other).is_eq()
        }
    }
}

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total_cmp(other)
    }
}

impl Hash for OrderedFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_nan() {
            state.write_u64(0x7ff8000000000000);
        } else {
            state.write_u64(self.to_bits());
        }
    }
}
