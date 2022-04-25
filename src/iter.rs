// SPDX-License-Identifier: MIT

use std::collections::binary_heap::{self, BinaryHeap};
use std::iter::FusedIterator;

use super::bucket::SortedBucket;


/// [Iterator] yielding items in descending order
///
/// This [Iterator] will yield an item only after all items greater have been
/// yielded.
///
/// # Time complexity
///
/// The implementation of [Iterator::next] has an amortized time complexity of
/// O(log(_n_/_b_)) with _n_ denoting the number of items and _b_ denoting the
/// bucket size the instance was constructed with, under the assuption that the
/// distribution of values amongst buckets is uniform. Draining the entire
/// [Iterator] thus has an expected runtime complexity of O(log(_n_/_b_)*_n_).
///
/// # Other notes
///
/// The omission of an implementation of [Clone] for this type is on purpose, as
/// it is meant for large amounts of data.
pub struct Iter<T: Ord> {
    buckets: BinaryHeap<SortedBucket<T>>,
}

impl<T: Ord> From<BinaryHeap<SortedBucket<T>>> for Iter<T> {
    fn from(buckets: BinaryHeap<SortedBucket<T>>) -> Self {
        Self{buckets}
    }
}

impl<T: Ord> ExactSizeIterator for Iter<T> {}

impl<T: Ord> FusedIterator for Iter<T> {}

impl<T: Ord> Iterator for Iter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(mut bucket) = self.buckets.peek_mut() {
            if let Some(item) = bucket.next() {
                return Some(item)
            } else {
                binary_heap::PeekMut::pop(bucket);
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.buckets.iter().map(ExactSizeIterator::len).sum();
        (size, Some(size))
    }
}

