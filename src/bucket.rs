// SPDX-License-Identifier: MIT
//! Types representing individual buckets and related utilities

use std::cmp::Ordering;


/// A collection of items to be committed to a [SortBuf](super::SortBuf)
///
/// Users of the library will usually not use this type directly.
pub struct Bucket<T>(pub(crate) Vec<T>);


/// A sorted collection of items
///
/// This type wraps a [Vec] of items sorted in ascending order and implements
/// [Ord] based on its last element. The ordering amongst buckets of this type
/// is equivalent to the ordering of the maximum item in each bucket.
///
/// In addition, a `SortedBucket` functions as an [Iterator] yielding (and
/// removing) its elements from last to first, i.e. in reverse or descending
/// order.
pub(crate) struct SortedBucket<T: Ord>(Vec<T>);

impl<T: Ord> From<Bucket<T>> for SortedBucket<T> {
    fn from(Bucket(items): Bucket<T>) -> Self {
        items.into()
    }
}

impl<T: Ord> From<Vec<T>> for SortedBucket<T> {
    fn from(mut items: Vec<T>) -> Self {
        items.sort_unstable();
        Self(items)
    }
}

impl<T: Ord> ExactSizeIterator for SortedBucket<T> {}

impl<T: Ord> std::iter::FusedIterator for SortedBucket<T> {}

impl<T: Ord> Iterator for SortedBucket<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.0.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.len();
        (len, Some(len))
    }
}

impl<T: Ord> Ord for SortedBucket<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.0.last(), &other.0.last())
    }
}

impl<T: Ord> PartialOrd for SortedBucket<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.0.last(), &other.0.last())
    }
}

impl<T: Ord> Eq for SortedBucket<T> {}

impl<T: Ord> PartialEq for SortedBucket<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.0.last(), &other.0.last())
    }
}

