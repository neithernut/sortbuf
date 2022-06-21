// SPDX-License-Identifier: MIT
/// Tests

use super::*;

use std::cmp::Reverse;
use std::num::NonZeroUsize;

use rand::Rng;


/// Item type to use for testing
type Item = u64;


#[test]
fn inserter_simple() {
    let mut buf: SortBuf<_> = Default::default();
    {
        let mut extender = inserter::Inserter::new(&mut buf);
        extender.set_bucket_size(NonZeroUsize::new(1000).expect("Failed to construct bucket size"));
        extender.extend(random_items(10_500));
    }

    let iter = buf.into_iter();
    assert_eq!(iter.len(), 10_500);
}


#[test]
fn inserter_half() {
    let mut buf: SortBuf<_> = Default::default();
    {
        let mut extender = inserter::Inserter::new(&mut buf);
        extender.set_bucket_size(NonZeroUsize::new(1000).expect("Failed to construct bucket size"));
        extender.extend(random_items(500));
    }

    let iter = buf.into_iter();
    assert_eq!(iter.len(), 500);
}


#[test]
fn inserter_exact() {
    let mut buf: SortBuf<_> = Default::default();
    {
        let mut extender = inserter::Inserter::new(&mut buf);
        extender.set_bucket_size(NonZeroUsize::new(1000).expect("Failed to construct bucket size"));
        extender.extend(random_items(10_000));
    }

    let iter = buf.into_iter();
    assert_eq!(iter.len(), 10_000);
}


#[test]
fn inserter_multiple() {
    let mut buf: SortBuf<_> = Default::default();
    {
        let mut extender = inserter::Inserter::new(&mut buf);
        extender.set_bucket_size(NonZeroUsize::new(1000).expect("Failed to construct bucket size"));
        extender.extend(random_items(500));
        extender.extend(random_items(1000));
    }

    let iter = buf.into_iter();
    assert_eq!(iter.len(), 1500);
}


#[test]
fn iter_sorted() {
    let mut items = random_items(10_500);

    let iter: iter::Iter<Item> = std::iter::from_fn(
        move || Some(items.by_ref().take(1000).collect::<Vec<_>>())
    ).take_while(|v| !v.is_empty()).map(bucket::Bucket::new).map(Into::into).collect::<Vec<_>>().into();

    assert_sorted(iter.map(Reverse))
}

#[test]
fn iter_count() {
    let mut items = random_items(10_500);

    let iter: iter::Iter<Item> = std::iter::from_fn(
        move || Some(items.by_ref().take(1000).collect::<Vec<_>>())
    ).take_while(|v| !v.is_empty()).map(bucket::Bucket::new).map(Into::into).collect::<Vec<_>>().into();

    assert_eq!(iter.count(), 10_500)
}


#[test]
fn bucket_sorted() {
    let bucket = bucket::Bucket::new(random_items(1000).collect::<Vec<_>>());
    assert_sorted(bucket::SortedBucket::from(bucket).map(Reverse))
}


/// Construct an [Iterator] yielding `num` random items
fn random_items(num: usize) -> impl Iterator<Item = Item> {
    let mut rng = rand_pcg::Mcg128Xsl64::new(0xcafef00dd15ea5e5); // seed taken from rand_pcg docs
    std::iter::from_fn(move || Some(rng.gen::<Item>())).take(num)
}

/// Check whether the given [Iterator] is sorted
fn assert_sorted<T: Ord>(mut iter: impl Iterator<Item = T>) {
    if let Some(first) = iter.next() {
        iter.try_fold(first, |c, n| if c <= n { Some(n) } else { None })
            .expect("Iterator does not yield sorted items");
    }
}

