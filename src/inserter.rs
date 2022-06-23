// SPDX-License-Identifier: MIT
//! Types and utilites for adding items to a [SortBuf](super::SortBuf)

use super::SortBuf;
use super::bucket::{self, Bucket};
use super::error::{InsertionError, InsertionResult};

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
    ///
    /// This function adds the given [Bucket] to the accumulator. If adding the
    /// [Bucket] failed due to an (re-)allocation failure, an error is returned
    /// alongside the bucket which could not be added.
    fn add_bucket(&mut self, buckets: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>>;

    /// Create an [Inserter] for this accumulator
    ///
    /// Create a new [Inserter] for this accumulator. [Bucket]s committed though
    /// the [Inserter] returned will be of a size near a
    /// [default bucket size](bucket::DEFAULT_BUCKET_BYTESIZE).
    fn inserter(self) -> Inserter<Self> where Self: Sized {
        Inserter::new(self)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for &mut A {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        (*self).add_bucket(bucket)
    }
}

impl<T: Ord> BucketAccumulator for SortBuf<T> {
    type Item = T;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        match self.buckets.try_reserve(1) {
            Ok(_)   => Ok(self.buckets.push(bucket.into())),
            Err(e)  => Err((e.into(), bucket)),
        }
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Mutex<A> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        self.lock().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Arc<Mutex<A>> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        self.lock().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for RwLock<A> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        self.write().expect("Could not lock mutex!").add_bucket(bucket)
    }
}

impl<A: BucketAccumulator> BucketAccumulator for Arc<RwLock<A>> {
    type Item = A::Item;

    fn add_bucket(&mut self, bucket: Bucket<Self::Item>) -> InsertionResult<Bucket<Self::Item>> {
        self.write().expect("Could not lock mutex!").add_bucket(bucket)
    }
}


/// Item feeder for [BucketAccumulator]s
///
/// Instances of this type allow collecting items into [Bucket]s and committing
/// them to a [BucketAccumulator] via the [insert_items](Self::insert_items)
/// function.
///
/// # Time complexity
///
/// A call to [insert_items](Self::insert_items) comes with an estimated runtime
/// cost of O(_n_ log(_b_) + _a_(_n_/_b_)) with _n_ denoting the number of items
/// to insert, _b_ denoting the target bucket size the instance was constructed
/// with and _a(x)_ denoting the complexity of adding _x_ buckets to the
/// [BucketAccumulator]. Since the influence of the second term will be
/// neglectible for sufficiently large _b_ and all relevant implementations, the
/// estimated runtime cost is effectifely O(_n_ log(_b_)).
///
/// # Bucket target size
///
/// While the above indicates that insertion is more costly with larget bucket
/// sizes, the _overall_ sorting performance benefits from larger buckets.
///
/// An `Inserter` fills [Bucket]s up to a target bucket size. A new `Inserter`
/// is initialized with a [default value](bucket::DEFAULT_BUCKET_BYTESIZE) which
/// is chosen to be safe in most situations, i.e. a value which is unlikely to
/// promote exhausting or overcomitting memory. However, for better performance
/// users of this type are encouraged to choose a target bucket size based on
/// the availible memory and the number of `Inserter`s involved in the target
/// use-case.
///
#[derive(Debug)]
pub struct Inserter<A: BucketAccumulator> {
    item_accumulator: Vec<A::Item>,
    bucket_accumulator: A,
    bucket_size: NonZeroUsize,
}

impl<A: BucketAccumulator> Inserter<A> {
    /// Create a new `Inserter` with a default bucket target size
    ///
    /// Create a new `Inserter` for the given `bucket_accumulator`. [Bucket]s
    /// committed to that [BucketAccumulator] will be of a size near a
    /// [default bucket size](bucket::DEFAULT_BUCKET_BYTESIZE).
    pub fn new(bucket_accumulator: A) -> Self {
        let bucket_size = Self::size_from_bytesize(bucket::DEFAULT_BUCKET_BYTESIZE);
        Self{item_accumulator: Default::default(), bucket_accumulator, bucket_size}
    }

    /// Insert items into the accumulator
    ///
    /// This function inserts the given `items` to the buffer. If the insertion
    /// fails due to an (re-)allocation failure, an error is returned.
    ///
    /// Even in the event of such an error, all items consumed from the
    /// `Iterator` passed to this method will reside either in the underlying
    /// [BucketAccumulator] or the `Inserter`s internal accumulator after the
    /// operation. Thus, callers can recover from allocation failures without
    /// any data loss by passing a mutable reference to an [Iterator] rather
    /// than a value, e.g. the result of [Iterator::by_ref].
    pub fn insert_items(&mut self, items: impl IntoIterator<Item = A::Item>) -> Result<(), InsertionError> {
        let mut items = items.into_iter().fuse();

        // The bucket size may have changed since the last attempt to insert
        // items. We don't want to grow buckets (or accumulators) after their
        // creation, as the reallocation might be costly. Shrinking, however,
        // should be unproblematic.
        let bucket_size = self.bucket_size.get();
        self.item_accumulator.shrink_to(bucket_size);

        // We first try to fill the current bucket to its capacity.
        let head_room = self.item_accumulator.capacity().saturating_sub(self.item_accumulator.len());
        self.item_accumulator.extend(items.by_ref().take(head_room));

        // As long as we get full buckets worth of items out of the iterator, we
        // have buckets to add to the target buffer.
        while self.item_accumulator.len() >= self.item_accumulator.capacity() {
            let bucket = Bucket::new(std::mem::take(&mut self.item_accumulator));
            if bucket.len() > 0 {
                self.bucket_accumulator.add_bucket(bucket).map_err(|(e, b)| {
                    self.item_accumulator = b.into_inner();
                    e
                })?
            }

            self.item_accumulator.try_reserve(bucket_size)?;
            self.item_accumulator.extend(items.by_ref().take(self.item_accumulator.capacity()));
        }

        Ok(())
    }

    /// Set a new target bucket size
    ///
    /// After calling this function, this inserter will commit [Bucket]s
    /// containing near `size` items.
    pub fn set_bucket_size(&mut self, size: NonZeroUsize) -> &mut Self {
        self.bucket_size = size;
        self
    }

    /// Set a new target bucket size in bytes
    ///
    /// After calling this function, this inserter will commit [Bucket]s near
    /// `bytesize` bytes in size.
    pub fn set_bucket_bytesize(&mut self, bytesize: usize) -> &mut Self {
        self.bucket_size = Self::size_from_bytesize(bytesize);
        self
    }

    /// Get the current target bucket size in items
    pub fn bucket_size(&self) -> NonZeroUsize {
        self.bucket_size
    }

    /// Get the current target bucket size in bytes
    pub fn bucket_bytesize(&self) -> usize {
        self.bucket_size.get() * std::mem::size_of::<A::Item>()
    }

    /// Determine the bucket target size for a given bytesize
    fn size_from_bytesize(bytesize: usize) -> NonZeroUsize {
        NonZeroUsize::new(bytesize / std::mem::size_of::<A::Item>())
            .or(NonZeroUsize::new(1))
            .expect("Could not compute bucket size")
    }
}

impl<A: BucketAccumulator<Item = std::cmp::Reverse<T>>, T: Ord> Inserter<A> {
    /// Insert reversed items into the accumulator
    ///
    /// This function inserts the given `items` to the buffer, each wrapped in
    /// a [std::cmp::Reverse]. If the insertion fails due to an (re-)allocation
    /// failure, an error is returned alongside an iterator over those items
    /// that were not inserted.
    pub fn insert_items_reversed(
        &mut self,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), InsertionError> {
        self.insert_items(items.into_iter().map(std::cmp::Reverse))
    }
}

impl<A: BucketAccumulator> Extend<A::Item> for Inserter<A> {
    fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iter: I) {
        self.insert_items(iter).expect("Failed to insert items")
    }
}

impl<A: BucketAccumulator> Drop for Inserter<A> {
    fn drop(&mut self) {
        let acc = std::mem::take(&mut self.item_accumulator);
        if !acc.is_empty() {
            self.bucket_accumulator
                .add_bucket(Bucket::new(acc))
                .expect("Failed to add final bucket")
        }
    }
}

