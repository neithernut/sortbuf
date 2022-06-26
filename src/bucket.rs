// SPDX-License-Identifier: MIT
//! Types representing individual buckets and related utilities

use std::alloc::{Allocator, Global};
use std::cmp::Ordering;
use std::fmt;


/// Default size for [Bucket]s
///
/// This constant holds a default size for buckets, in bytes. The constant
/// is choosen to be reasonably large without obstructing the library's use
/// on smaller machines. Currently, it is set to 16MB.
///
/// The rationale behind that value is that on a typical SBC with a quadcore
/// and 1GB of ram, it should be possible to accumulate items into buckets
/// for multiple (e.g. 3) buffers on all cores without exhausting memory (or
/// running into overcommitting).
pub const DEFAULT_BUCKET_BYTESIZE: usize = 16*1024*1024;


/// A collection of items to be committed to a [SortBuf](super::SortBuf)
///
/// Users of the library will usually not use this type directly.
///
/// # Other notes
///
/// The omission of an implementation of [Clone] for this type is on purpose, as
/// it holds non-shared ownership over significant amounts of data.
pub struct Bucket<T, A: Allocator = Global>(Vec<T, A>);

impl<T: Ord, A: Allocator> Bucket<T, A> {
    /// Create a bucket from a [Vec] of items
    ///
    /// # Time complexity
    ///
    /// Construction of a sorted bucket involves sorting the items. Thus, it
    /// comes with a run-time cost of O(_b_*log(_b_)) with bucket size _b_.
    pub(crate) fn new(mut items: Vec<T, A>) -> Self {
        items.shrink_to_fit();
        items.sort_unstable();
        Self(items)
    }

    /// Convert this bucket back to a [Vec]
    pub(crate) fn into_inner(self) -> Vec<T, A> {
        self.0
    }

    /// Retrieve the number of items in this bucket
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T: Ord, A: Allocator> fmt::Debug for Bucket<T, A> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Bucket({} items)", self.len())
    }
}


/// A sorted collection of items
///
/// This type wraps a [Vec] of items sorted in ascending order and implements
/// [Ord] based on its last element. The ordering amongst buckets of this type
/// is equivalent to the ordering of the maximum item in each bucket.
///
/// In addition, a `SortedBucket` functions as an [Iterator] yielding (and
/// removing) its elements from last to first, i.e. in reverse or descending
/// order.
///
/// # Other notes
///
/// The omission of an implementation of [Clone] for this type is on purpose, as
/// it holds non-shared ownership over significant amounts of data.
pub(crate) struct SortedBucket<T: Ord, A: Allocator>(Vec<T, A>);

impl<T: Ord, A: Allocator> SortedBucket<T, A> {
    /// Retrieve the current overcapacity of this bucket
    ///
    /// The overcapacity is defined as the number of additional items the inner
    /// [Vec] has capacity for.
    #[inline(always)]
    pub fn overcapacity(&self) -> usize {
        self.0.capacity() - self.0.len()
    }

    /// Shrink the inner [Vec] to the number of items it currently holds
    ///
    /// This operation sheds overcapacity.
    #[inline(always)]
    pub fn shink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }
}

impl<T: Ord, A: Allocator> From<Bucket<T, A>> for SortedBucket<T, A> {
    fn from(Bucket(items): Bucket<T, A>) -> Self {
        Self(items)
    }
}

impl<T: Ord, A: Allocator> ExactSizeIterator for SortedBucket<T, A> {}

impl<T: Ord, A: Allocator> std::iter::FusedIterator for SortedBucket<T, A> {}

impl<T: Ord, A: Allocator> Iterator for SortedBucket<T, A> {
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<T> {
        self.0.pop()
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.len();
        (len, Some(len))
    }
}

impl<T: Ord, A: Allocator> Ord for SortedBucket<T, A> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.0.last(), &other.0.last())
    }
}

impl<T: Ord, A: Allocator> PartialOrd for SortedBucket<T, A> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.0.last(), &other.0.last())
    }
}

impl<T: Ord, A: Allocator> Eq for SortedBucket<T, A> {}

impl<T: Ord, A: Allocator> PartialEq for SortedBucket<T, A> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.0.last(), &other.0.last())
    }
}

impl<T: Ord, A: Allocator> fmt::Debug for SortedBucket<T, A> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "SortedBucket({} items)", self.len())
    }
}

