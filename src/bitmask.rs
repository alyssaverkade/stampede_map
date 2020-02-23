use std::arch::x86_64::*;
use std::ops::{BitOr, Deref};

pub struct BitMask {
    mask: u16,
}

impl BitMask {
    #[inline(always)]
    pub fn new(mask: u16) -> Self {
        Self { mask }
    }

    #[inline(always)]
    #[cfg(target_feature = "sse3")]
    /// Load a vector of length 16 into a SSE register and constructs a bitmask of all
    /// the values that match `predicate`
    ///
    pub fn matches(vec: [u8; 16], predicate: u8) -> Self {
        unsafe {
            let vec: __m128i = _mm_lddqu_si128(vec.as_ptr() as *const __m128i);
            let pred = _mm_set1_epi8(predicate as i8);
            BitMask::new(_mm_movemask_epi8(_mm_cmpeq_epi8(vec, pred)) as u16)
        }
    }
}

#[cfg(not(feature = "nightly"))]
impl Iterator for BitMask {
    type Item = u16;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }
        let result = self.mask;
        self.mask &= self.mask - 1;
        Some(result.trailing_zeros() as Self::Item)
    }
}

#[cfg(feature = "nightly")]
impl Iterator for BitMask {
    type Item = u16;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }
        let result = self.mask;
        self.mask &= self.mask - 1;
        // Intrinsics are implementation defined
        Some(unsafe { core::intrinsics::cttz_nonzero(result) } as Self::Item)
    }
}

impl Into<bool> for BitMask {
    #[inline(always)]
    fn into(self) -> bool {
        self.mask != 0
    }
}

impl Deref for BitMask {
    type Target = u16;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.mask
    }
}

impl BitOr for BitMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        BitMask::new(self.mask | rhs.mask)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bitmask_iteration() {
        let val: u16 = 11;
        let mask = BitMask::new(11);
        let vals: Vec<u16> = mask.map(|x| x as u16).collect::<Vec<u16>>();
        assert_eq!(vals[0], 0);
        assert_eq!(vals[1], 1);
        assert_eq!(vals[2], 3);
    }

    #[test]
    fn test_bitmask_empty_matches() {
        let mut vec = [0; 16];

        for i in 0u8..16 {
            vec[i as usize] = i
        }
        let mask = BitMask::matches(vec, (-128i8) as u8);
        assert_eq!(*mask, 0);
    }
}
