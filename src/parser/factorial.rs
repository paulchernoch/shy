use std::convert::TryFrom;

//..................................................................

/// Factorial lookup for integers.

lazy_static! {
    static ref FACTORIAL_FIXED: [i64; 21] = {
        let factorial_values : [i64; 21] = [
            1,
            1,
            1 * 2,
            1 * 2 * 3,
            1 * 2 * 3 * 4,
            1 * 2 * 3 * 4 * 5,
            1 * 2 * 3 * 4 * 5 * 6,
            1 * 2 * 3 * 4 * 5 * 6 * 7,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18 * 19,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18 * 19 * 20
        ];
        factorial_values
    };
}

/// Compute the factorial of an integer between 0 and 20 (inclusive).
/// If the number os out of range, return None.
pub fn factorial(n : i64) -> Option<i64> {
    match usize::try_from(n) {
        Ok(i) if i <= 20 => Some(FACTORIAL_FIXED[i]),
        _ => None
    }
}

//..................................................................

/// Approximate Factorial lookup for numbers up to and including 170!.

lazy_static! {
    static ref FACTORIAL_FLOAT: [f64; 171] = {
        let mut factorial_values : [f64; 171] = [1.0; 171];
        for n in 2..=170 {
            factorial_values[n] = factorial_values[n-1] * (n as f64);
        }
        factorial_values
    };
}

/// Compute the factorial of an integer between 0 and 170 (inclusive), where the larger values are approximate.
/// If the number is out of range, return None.
pub fn factorial_approx(n : i64) -> Option<f64> {
    match usize::try_from(n) {
        Ok(i) if i <= 170 => Some(FACTORIAL_FLOAT[i]),
        _ => None
    }
}


