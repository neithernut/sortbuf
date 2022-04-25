// SPDX-License-Identifier: MIT
/// Tests

use super::*;

use rand::Rng;


/// Item type to use for testing
type Item = u64;


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

