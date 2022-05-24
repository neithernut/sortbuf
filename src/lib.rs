// SPDX-License-Identifier: MIT

mod bucket;
mod extender;
mod iter;

#[cfg(test)]
mod tests;


pub use bucket::Bucket;
pub use extender::{BucketAccumulator, Extender};


/// Data structure for preparing a large number of items for sorted iteration
///
/// This data structure buffers items for later iteration in descending order.
/// New items are inserted via an [Extender] which has to be constructed
/// separately for a given buffer. Once all (or sufficently many) items are
/// inserted, [IntoIterator] may be used for iterating over these items in
/// descending order (according to the items' implementation of [Ord]).
///
/// For ascending iteration, users need to wrap items in [std::cmp::Reverse] and
/// unwrap them during the final iteration.
///
/// # Other notes
///
/// The omission of an implementation of [Clone] for this type is on purpose, as
/// it is meant for large amounts of data.
#[derive(Default)]
pub struct SortBuf<T: Ord> {
    buckets: std::collections::BinaryHeap<bucket::SortedBucket<T>>,
}

impl<T: Ord> IntoIterator for SortBuf<T> {
    type Item = T;
    type IntoIter = iter::Iter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.buckets.into()
    }
}

