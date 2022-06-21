// SPDX-License-Identifier: MIT
//! Sort a large number of items in memory
//!
//! This library provides types, most prominently [SortBuf], and traits for
//! accumulating a large number of items in memory and iterating over them in
//! ascending or descending order (as per [Ord]). The implementation
//!
//! * avoids potentially costly reallocations,
//! * releases chunks of memory every now and then during iteration,
//! * doesn't introduce much memory overhead,
//! * supports multi-threaded insertion and
//! * isn't awfully slow.
//!
//! # Examples
//!
//! Natively, [SortBuf] will prepare items for descending iteration:
//!
//! ```
//! let mut sortbuf = sortbuf::SortBuf::new();
//! let mut inserter = sortbuf::Inserter::new(&mut sortbuf);
//! inserter.insert_items([10, 20, 5, 17]).map_err(|(e, _)| e).expect("Failed to insert items");
//! drop(inserter);
//! assert!(sortbuf.into_iter().eq([20, 17, 10, 5]));
//! ```
//!
//! For ascending iteration, items need to be wrapped in [std::cmp::Reverse]:
//!
//! ```
//! let mut sortbuf = sortbuf::SortBuf::new();
//! let mut inserter = sortbuf::Inserter::new(&mut sortbuf);
//! inserter
//!     .insert_items([10, 20, 5, 17].into_iter().map(std::cmp::Reverse))
//!     .map_err(|(e, _)| e)
//!     .expect("Failed to insert items");
//! drop(inserter);
//! assert!(sortbuf.unreversed().eq([5, 10, 17, 20]));
//! ```
//!
//! Multithreaded insertion is supported via multiple [Inserter]s:
//!
//! ```
//! use std::sync::{Arc, Mutex};
//! let sortbuf: Arc<Mutex<sortbuf::SortBuf<_>>> = Default::default();
//! let workers: Vec<_> = (0..4).map(|n| {
//!     let mut inserter = sortbuf::Inserter::new(sortbuf.clone());
//!     std::thread::spawn(move || inserter
//!         .insert_items((0..1000).map(|i| 4*i+n))
//!         .map_err(|(e, _)| e)
//!         .expect("Failed to insert items"))
//! }).collect();
//! workers.into_iter().try_for_each(|h| h.join()).unwrap();
//! assert!(sortbuf.lock().unwrap().take().into_iter().eq((0..4000).rev()));
//! ```
//!
//! # Approach and comparison
//!
//! As indicated in the examples above, adding new items to a buffer is done via
//! [Inserter]s. These accumulate items in pre-sorted [Bucket]s and commit them
//! to their target buffer. Later, that buffer can be converted to an [Iterator]
//! which yields items taken from those [Bucket]s, which involves selecting the
//! [Bucket] with the current greatest item in the buffer.
//!
//! While a significant amount of time is spent during insertion, the majority
//! of time is usually spent during iteration. Performance is usually better,
//! and skews towards more time spent in the parallelizable insertion state,
//! with fewer, bigger [Bucket]s. As [Bucket]s are pre-allocated, this comes at
//! the cost of flexibility regarding memory.
//!
//! ## Comparison to Vec and sort
//!
//! Buffering and sorting items can also be done using a [Vec] (for buffering)
//! in conjunction with [slice::sort], [slice::sort_unstable] or another sorting
//! function. The process is then usually split into an insertion, a sorting and
//! an iteration phase, with sort being the most computational intensive phase.
//!
//! Sorting and iteration over the items in a [Vec] is generally faster than
//! with the utilities provided by this library ---in the single-threaded case.
//! However, this library does allow insertion from multiple threads, contrary
//! to a bare [Vec]. In addition, the performance of inserting items into a
//! [Vec] hinges on the reallocation performance, which might be poor in some
//! cases, e.g. if multiple buffers are involved.
//!
//! The need of a single, separate, computational intensive sorting phase may
//! also have some implications on overall performance in some use-cases. With
//! the types provided by this library, sorting will likely interleave with I/O
//! linked to insertion and/or the final iteration, spread out over the entire
//! process. Thus, the underlying OS may have more opportunities to perform
//! background operations related to reads (insertion stage) and writes
//! (iteration stage), increasing the overall throughput.
//!
//! ## Comparison to BTreeSet
//!
//! Another option for sorting items without the need for a separate sorting
//! phase would be an [BTreeSet](std::collections::BTreeSet). Contrary to the
//! `sortbuf` approach, most of the time is spent in the insertion phase rather
//! than the iteration phase. Using a [BTreeSet](std::collections::BTreeSet) is
//! usually slower than a [SortBuf] with sufficiently large [Bucket]s, not
//! parallelizable and incurs a higher memory overhead.

mod bucket;
mod extender;
mod iter;

pub mod error;

#[cfg(test)]
mod tests;


pub use bucket::{Bucket, DEFAULT_BUCKET_BYTESIZE};
pub use extender::{BucketAccumulator, Inserter};


/// Data structure for preparing a large number of items for sorted iteration
///
/// This data structure buffers items for later iteration in descending order.
/// New items are inserted via an [Inserter] which has to be constructed
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
/// items via [Inserter]s is O(_n_ log(_b_)). The estimated runtime cost of
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

    /// Take this buffer's contents, leaving an empty buffer
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
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

