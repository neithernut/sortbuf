// SPDX-License-Identifier: MIT
//! Types and utilites for adding items to a [SortBuf](super::SortBuf)

use super::{SortBuf, bucket::Bucket};


/// Accumulator for [Bucket]s
///
/// Implementations of this type allow accumulating [Bucket]s, usually with the
/// goal of producing an [Iterator] yielding items in ascending or descending
/// order.
///
/// Users will usually not implement this trait but rely on implementations
/// provided by this library, such as [SortBuf].
pub trait BucketAccumulator {
    /// The type of items buckets contain
    type Item: Ord;

    /// Add new [Bucket]s to this accumulator
    fn add_buckets<I: Iterator<Item = Bucket<Self::Item>>>(&mut self, buckets: I);
}

impl<T: Ord> BucketAccumulator for &mut SortBuf<T> {
    type Item = T;

    fn add_buckets<I: Iterator<Item = Bucket<Self::Item>>>(&mut self, buckets: I) {
        self.buckets.extend(buckets.map(Into::into))
    }
}

