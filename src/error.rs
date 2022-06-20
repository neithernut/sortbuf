// SPDX-License-Identifier: MIT
//! Types and utilities related to error handling and reporting

use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;


/// Insertion error
///
/// This type conveys errors occuring during the insertion of items to a buffer.
#[derive(Debug)]
pub struct InsertionError(TryReserveError);

impl From<TryReserveError> for InsertionError {
    fn from(inner: TryReserveError) -> Self {
        Self(inner)
    }
}

impl Error for InsertionError {
    fn cause(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl fmt::Display for InsertionError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Could not add items to accumulator")
    }
}


/// Result type for insertions
///
/// In the case of an insertion failure, we need to return an `InsertionError`.
/// However, we also want to return the item(s) we failed to insert in some way.
pub type InsertionResult<T> = Result<(), (InsertionError, T)>;

