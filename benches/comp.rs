//! Comparison between different kinds of sorting buffers

use std::time::Duration;


const MAX_M_ITEMS: usize = 1024;


fn main() {
    println!("implementation | 2^20 Is | T wall  ");
    println!("---------------|---------|---------");

    let benches: [(_, &dyn Fn(usize) -> (Duration, Diff, Diff)); 1] = [
        ("baseline",    &|i| bench_func(baseline, i)),
    ];

    std::iter::successors(Some(1usize), |s| (*s).checked_mul(4))
        .take_while(|s| *s <= MAX_M_ITEMS)
        .flat_map(|s| benches.iter().map(move |(n, b)| (n, s, b)))
        .for_each(|(n, s, b)| {
            let (t, d1, d2) = b(s*1024*1024);
            println!(
                "{:<15}|{:>9}|{:>9}",
                n,
                s,
                t.as_millis(),
            )
        });
}


fn baseline(num: usize) -> impl IntoIterator<Item=u64> {
    let mut curr = Default::default();
    random_items(num).map(move |v| {
        curr = v.saturating_add(curr);
        curr
    })
}


/// Benchmark a single sorting buffer
///
/// This function accepts a function which is expected to construct a sorted
/// buffer with random data as well as the number of items with which to fill
/// the buffer.
///
/// The function will construct the sorted buffer, measuring the resources used
/// suring the process. It will then drain the buffer, again measuring the
/// resource usage. Both resource usages are then returned to the caller
/// together with the overall wallclock time.
fn bench_func<I>(mk_iter: impl Fn(usize) -> I, items: usize) -> (Duration, Diff, Diff)
where I: IntoIterator<Item = u64>,
{
    let t0 = std::time::Instant::now();
    let s0 = Snapshot::new();
    let iter = mk_iter(items);

    let s1 = Snapshot::new();
    let mut iter = iter.into_iter();
    if let Some(first) = iter.next() {
        // TODO: use a black box of some sort to prevent the draining to be optimized out
        iter.try_fold(first, |c, n| if c <= n { Some(n) } else { None })
            .expect("Iterator does not yield sorted items");
    }

    let s2 = Snapshot::new();

    (t0.elapsed(), s1.diff(s0), s2.diff(s1))
}


/// Create an iterator over random items
fn random_items(num: usize) -> impl Iterator<Item = u64> {
    random_items_with_seed(num, 0xcafef00dd15ea5e5) // seed taken from rand_pcg docs
}


/// Create an iterator over random items with a given seed
fn random_items_with_seed(num: usize, seed: u128) -> impl Iterator<Item = u64> {
    use rand::Rng;

    let mut rng = rand_pcg::Mcg128Xsl64::new(seed);
    std::iter::from_fn(move || Some(rng.gen())).take(num)
}


/// Snapshot of the current resource usage and time
#[derive(Copy, Clone, Debug)]
struct Snapshot {
}

impl Snapshot {
    /// Create a new [Snapshot]
    pub fn new() -> Self {
        Self{}
    }

    /// Compare this [Snapshot] to an earlier one
    pub fn diff(&self, older: Self) -> Diff {
        Diff{}
    }
}


/// The difference between two [Snapshot]s
#[derive(Copy, Clone, Debug)]
struct Diff {
}

