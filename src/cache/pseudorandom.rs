use std::sync::atomic::{AtomicI64, Ordering};

lazy_static! {
    static ref PHI: f64 = {
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        phi
    };
}

#[derive(Debug)]
/// Pseudo-random number generator based on using the Golden Ratio Phi
/// in a Kronecker Recurrence sequence for S0 = 0.
/// 
///   Phi is the golden ratio: (1 + √5)/2
/// 
/// See http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
/// The paper describes the excellent properties of this sequence in one-dimensional problems.
/// It doesn't do very well in higher dimensions, but we only need good low-discrepancy for one dimension! 
/// (*Low-discrepancy* is the property that defines good pseudo-random sequences.)
pub struct PseudoRng {
    counter : AtomicI64,
    lower_bound : f64,
    upper_bound : f64
}


impl PseudoRng {
    /// Create a new Pseudo-random number generator that will produce numbers 
    /// between the given lower_bound (inclusive) and upper_bound (exclusive), starting with a given seed. 
    pub fn new_with_seed(lower_bound : i32, upper_bound : i32, seed : i32) -> PseudoRng {
        if lower_bound >= upper_bound {
            panic!("For PseudoRng, lower bound {} must not equal or exceed upper bound {}", lower_bound, upper_bound);
        }
        PseudoRng {
            lower_bound : lower_bound as f64,
            upper_bound : upper_bound as f64,
            counter : AtomicI64::new(seed as i64)
        }
    }

    /// Create a new Pseudo-random number generator that will produce integers 
    /// between zero and the given upper_bound (exclusive), starting with a given seed. 
    pub fn new(upper_bound : i32) -> PseudoRng {
        Self::new_with_seed(0, upper_bound, 0)
    }

    fn advance(&mut self) -> f64 {
        let n = self.counter.fetch_add(1, Ordering::SeqCst) as f64;
        ((n * *PHI).fract() * (self.upper_bound - self.lower_bound)) + self.lower_bound
    }

    /// Yield the next pseudorandom integer in the series.
    pub fn next_i64(&mut self) -> i64 {
        let mut r = self.advance();
        if r >= self.upper_bound { r = self.upper_bound - 1.0; }
        r as i64
    }

    /// Yield the next pseudorandom integer in the series.
    pub fn next_usize(&mut self) -> usize {
        let mut r = self.advance();
        if r >= self.upper_bound { r = self.upper_bound - 1.0; }
        r as usize
    }

    /// Yield the next pseudorandom float in the series.
    pub fn next_f64(&mut self) -> f64 {
        let mut r = self.advance();
        if r >= self.upper_bound { r = self.upper_bound - std::f64::EPSILON }
        r
    }


}