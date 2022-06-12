// SPDX-License-Identifier: MIT
//! Types and utilites for adding items to a [SortBuf](super::SortBuf)

use super::SortBuf;
use super::bucket::{self, Bucket};

use std::iter::FusedIterator;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, RwLock};


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

    /// Add a new [Bucket] to this accumulator
    fn add_bucket(&mut self, buckets: Bucket<Self::Item>);

    /// Create an [Extender] for this accumulator
    ///
    /// Create a new [Extender] for this accumulator. [Bucket]s committed though
    /// the [Extender] returned will be of a size near a
    /// [default bucket size](bucket::DEFAULT_BUCKET_BYTESIZE).
    fn extender(self) -> Extender<Self> where Self: Sized {
        Extender::new(self)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for &mut A {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        (*self).add_bucket(bucket)
    }
}

impl<T: Ord> BucketAccumulator for SortBuf<T> {
    type Item = T;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        self.buckets.push(bucket.into())
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Mutex<A> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        self.lock().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Arc<Mutex<A>> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        self.lock().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for RwLock<A> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        self.write().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Arc<RwLock<A>> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) {
        self.write().expect("Could not lock mutex!").add_bucket(bucket)
    }
}


/// Item feeder for [BucketAccumulator]s
///
/// Instances of this type allow collecting items into [Bucket]s and committing
/// them to a [BucketAccumulator]. In particular, this type implements [Extend].
///
/// # Time complexity
///
/// The implementation of [Extend::extend] comes with an estimated runtime cost
/// of O(_n_*log(_b_) + _a_(_n_/_b_)) with _n_ denoting the number of items by
/// which the `Extender` is extended, _b_ denoting the target bucket size the
/// instance was constructed with and _a(x)_ denoting the complexity of adding
/// _x_ buckets to the [BucketAccumulator]. Since the influence of the second
/// term will be neglectible for sufficiently large _b_ and all relevant
/// implementations, the estimated runtime cost is effectifely O(_n_*log(_b_)).
pub struct Extender<A: BucketAccumulator> {
    item_accumulator: Vec<A::Item>,
    bucket_accumulator: A,
    bucket_size: NonZeroUsize,
}

impl<A: BucketAccumulator> Extender<A> {
    /// Create a new `Extender` with a default bucket target size
    ///
    /// Create a new `Extender` for the given `bucket_accumulator`. [Bucket]s
    /// committed to that [BucketAccumulator] will be of a size near a
    /// [default bucket size](bucket::DEFAULT_BUCKET_BYTESIZE).
    pub fn new(bucket_accumulator: A) -> Self {
        let bucket_size = Self::size_from_bytesize(bucket::DEFAULT_BUCKET_BYTESIZE);
        Self{item_accumulator: Default::default(), bucket_accumulator, bucket_size}
    }

    /// Set a new target bucket size
    ///
    /// After calling this function, this extender will commit [Bucket]s
    /// containing ner `size` items.
    pub fn set_bucket_size(&mut self, size: NonZeroUsize) -> &mut Self {
        self.bucket_size = size;
        self
    }

    /// Set a new target bucket size in bytes
    ///
    /// After calling this function, this extender will commit [Bucket]s near
    /// `bytesize` bytes in size.
    pub fn set_bucket_bytesize(&mut self, bytesize: usize) -> &mut Self {
        self.bucket_size = Self::size_from_bytesize(bytesize);
        self
    }

    /// Determine the bucket target size for a given bytesize
    fn size_from_bytesize(bytesize: usize) -> NonZeroUsize {
        NonZeroUsize::new(bytesize / std::mem::size_of::<A::Item>())
            .or(NonZeroUsize::new(1))
            .expect("Could not compute bucket size")
    }
}

impl<A: BucketAccumulator> Extend<A::Item> for Extender<A> {
    fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iter: I) {
        BucketGen::initialize(
            &mut self.item_accumulator,
            self.bucket_size,
            iter.into_iter().fuse(),
        ).for_each(|b| self.bucket_accumulator.add_bucket(b))
    }
}

impl<A: BucketAccumulator> Drop for Extender<A> {
    fn drop(&mut self) {
        let acc = std::mem::take(&mut self.item_accumulator);
        if !acc.is_empty() {
            self.bucket_accumulator.add_bucket(Bucket::new(acc))
        }
    }
}


/// Iterator adapter for generating buckets
///
/// This [Iterator] yields [OrderedBuckets] of a fixed size from the items taken
/// from a wrapped an [Iterator]. Items are accumulated in a `Vec` which needs
/// to be supplied by upon creation of a generator by reference.
///
/// # Time complexity
///
/// The implementation of [Iterator::next] has an amortized time complexity of
/// O(log(_b_)) with _b_ denoting the bucket size the instance was constructed
/// with. Draining the entire [Iterator] thus has an expected time complexity
/// of O(_n_*log(_b_)) with _n_ being the number of items yielded by the item
/// source.
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

            // Creating a `Bucket` comes with the cost of sorting its items.
            Some(Bucket::new(std::mem::replace(self.accumulator, next_bucket)))
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

