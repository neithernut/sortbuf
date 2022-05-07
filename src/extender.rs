// SPDX-License-Identifier: MIT
//! Types and utilites for adding items to a [SortBuf](super::SortBuf)

use super::{SortBuf, bucket::Bucket};

use std::iter::FusedIterator;
use std::num::NonZeroUsize;


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


/// Iterator adapter for generating buckets
///
/// This [Iterator] yields [OrderedBuckets] of a fixed size from the items taken
/// from a wrapped an [Iterator]. Items are accumulated in a `Vec` which needs
/// to be supplied by upon creation of a generator by reference.
pub(crate) struct BucketGen<'a, T: Ord, I: FusedIterator<Item = T>> {
    accumulator: &'a mut Vec<T>,
    bucket_size: NonZeroUsize,
    item_source: I,
}

impl<'a, T: Ord, I: FusedIterator<Item = T>> BucketGen<'a, T, I> {
    /// Create a generator, initializing the given accumulator
    ///
    /// This function creates a bucket generator yielding buckets of the given
    /// `bucket_size` (number of items in a bucket). As a preparation, it tries
    /// to fill up the given accumulator with items. If the terget `bucket_size`
    /// is not reached during initialization, the resulting [Iterator] will not
    /// yield any buckets.
    pub fn initialize(accumulator: &'a mut Vec<T>, bucket_size: NonZeroUsize, mut item_source: I) -> Self {
        let head_room = bucket_size.get().saturating_sub(accumulator.len());
        accumulator.reserve(head_room);
        accumulator.extend(item_source.by_ref().take(head_room));

        Self{accumulator, bucket_size, item_source}
    }
}

impl<T: Ord, I: FusedIterator<Item = T>> FusedIterator for BucketGen<'_, T, I> {}

impl<T: Ord, I: FusedIterator<Item = T>> Iterator for BucketGen<'_, T, I> {
    type Item = Bucket<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let bucket_size = self.bucket_size.get();

        // We'll fill bucket after bucket until we drained iter dry. That point
        // we reach once we end up having room left in the current one.
        if self.accumulator.len() >= bucket_size {
            let next_bucket = self.item_source.by_ref().take(bucket_size).collect();
            let mut full_bucket = std::mem::replace(self.accumulator, next_bucket);
            full_bucket.shrink_to_fit();
            Some(Bucket(full_bucket))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let current_bucket = self.accumulator.len();
        let bucket_size = self.bucket_size.get();
        let (source_min, source_max) = self.item_source.size_hint();

        (
            (source_min.saturating_add(current_bucket) / bucket_size),
            source_max.and_then(|s| s.checked_add(current_bucket)).map(|s| (s / bucket_size))
        )
    }
}

