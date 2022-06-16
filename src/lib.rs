// SPDX-License-Identifier: MIT

mod bucket;
mod extender;
mod iter;

#[cfg(test)]
mod tests;


pub use bucket::{Bucket, DEFAULT_BUCKET_BYTESIZE};
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
/// # Time complexity
///
/// Assuming a fixed bucket size _b_, the estimated runtime cost of inserting _n_
/// items via [Extender]s is O(_n_ log(_b_)). The estimated runtime cost of
/// draining the [Iterator] provided through this type's [IntoIterator] impl is
/// O(_n_ log(_n_/_b_)).
///
/// In practice, the runtime cost will be higher for draining the iterator than
/// for the insertion. The iteration performance will be significantly affected
/// by the term log(_n_/_b_), since it will be reflected in the average number
/// of cache-misses involved in retrieving a single item. Thus, greater values
/// of _b_ are to be preferred. As a rule of thumb, aim for a number of buckets
/// (i.e. _n_/_b_) well under 100 for ok-ish performance.
///
/// Note also that the cost of insertion can be split between multiple threads,
/// if the [SortBuf] is wrapped in a mutex for which the [BucketAccumulator]
/// trait is implemented.
///
/// # Other notes
///
/// The omission of an implementation of [Clone] for this type is on purpose, as
/// it is meant for large amounts of data.
pub struct SortBuf<T: Ord> {
    buckets: Vec<bucket::SortedBucket<T>>,
}

impl<T: Ord> SortBuf<T> {
    /// Create a new sorting buffer
    pub fn new() -> Self {
        Self {buckets: Vec::new()}
    }
}

impl<T: Ord> Default for SortBuf<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> SortBuf<std::cmp::Reverse<T>> {
    /// Convert into an [Iterator] over items unwrapped from [std::cmp::Reverse]
    ///
    /// This funtion allows convenient retrieval of the buffered items in their
    /// unreversed order. Use this function if you need an iterator over items
    /// in ascending order.
    pub fn unreversed(self) -> impl Iterator<Item = T> {
        self.into_iter().map(|std::cmp::Reverse(v)| v)
    }
}

impl<T: Ord> IntoIterator for SortBuf<T> {
    type Item = T;
    type IntoIter = iter::Iter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.buckets.into()
    }
}

