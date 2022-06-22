// SPDX-License-Identifier: MIT
//! Comparison between different kinds of sorting buffers

use std::sync::atomic;
use std::time::Duration;


const NUM_THREADS: usize = 4;
const MAX_M_ITEMS: usize = 1024;


fn main() {
    println!("implementation | 2^20 Is | T wall  | T_f usr | T_f sys | T_d usr | T_d sys | mem     ");
    println!("---------------|---------|---------|---------|---------|---------|---------|---------");


    let benches: [(_, &dyn Fn(usize) -> (Duration, Diff, Diff)); 6] = [
        ("baseline",        &|i| bench_func(baseline, i)),
        ("vec",             &|i| bench_func(fill_vec, i)),
        ("btree",           &|i| bench_func(fill_btree, i)),
        ("sortbuf",         &|i| bench_func(fill_sortbuf, i)),
        ("sortbuf jumbo",   &|i| bench_func(fill_sortbuf_jumbo, i)),
        ("sortbuf 4t",      &|i| bench_func(fill_sortbuf_threads, i)),
    ];

    std::iter::successors(Some(1usize), |s| (*s).checked_mul(4))
        .take_while(|s| *s <= MAX_M_ITEMS)
        .flat_map(|s| benches.iter().map(move |(n, b)| (n, s, b)))
        .for_each(|(n, s, b)| {
            let (t, d1, d2) = b(s*1024*1024);
            println!(
                "{:<15}|{:>9}|{:>9}|{:>9}|{:>9}|{:>9}|{:>9}|{:>9}",
                n,
                s,
                t.as_millis(),
                d1.user_time.as_millis(),
                d1.system_time.as_millis(),
                d2.user_time.as_millis(),
                d2.system_time.as_millis(),
                d1.allocated / (1024*1024),
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


fn fill_vec(num: usize) -> impl IntoIterator<Item=u64> {
    let mut buf: Vec<_> = random_items(num).collect();
    buf.sort_unstable();
    buf
}


fn fill_btree(num: usize) -> impl IntoIterator<Item=u64> {
    random_items(num).collect::<std::collections::BTreeSet<_>>()
}


fn fill_sortbuf(num: usize) -> impl IntoIterator<Item=u64> {
    let mut buf: sortbuf::SortBuf<_> = Default::default();

    let mut inserter = sortbuf::Inserter::new(&mut buf);
    inserter.insert_items_reversed(random_items(num)).map_err(|(e, _)| e).expect("Error while inserting");
    std::mem::drop(inserter);

    buf.unreversed()
}


fn fill_sortbuf_jumbo(num: usize) -> impl IntoIterator<Item=u64> {
    let mut buf: sortbuf::SortBuf<_> = Default::default();

    let mut inserter = sortbuf::Inserter::new(&mut buf);
    inserter.set_bucket_bytesize(sortbuf::DEFAULT_BUCKET_BYTESIZE * 4);
    inserter.insert_items_reversed(random_items(num)).map_err(|(e, _)| e).expect("Error while inserting");
    std::mem::drop(inserter);

    buf.unreversed()
}


fn fill_sortbuf_threads(num: usize) -> impl IntoIterator<Item=u64> {
    use std::sync::{Arc, Mutex};

    let buf: Arc<Mutex<sortbuf::SortBuf<std::cmp::Reverse<u64>>>> = Default::default();

    random_items(NUM_THREADS).map(|seed| {
        let mut inserter = sortbuf::Inserter::new(buf.clone());
        std::thread::spawn(move || inserter.insert_items_reversed(
            random_items_with_seed(num / NUM_THREADS, seed.into())
        ).map_err(|(e, _)| e).expect("Error while inserting"))
    }).collect::<Vec<_>>().into_iter().try_for_each(|h| h.join()).expect("Error while waiting for threads");

    Arc::try_unwrap(buf)
        .map_err(|_| ())
        .and_then(|m| m.into_inner().map_err(|_| ()))
        .expect("Failed to unwrap buffer!")
        .unreversed()
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
    user_time: Duration,
    system_time: Duration,
    allocated: usize,
}

impl Snapshot {
    /// Create a new [Snapshot]
    pub fn new() -> Self {
        let mut rusage = libc::rusage{
            ru_utime:       libc::timeval{tv_sec: 0, tv_usec: 0},
            ru_stime:       libc::timeval{tv_sec: 0, tv_usec: 0},
            ru_maxrss:      0,
            ru_ixrss:       0,
            ru_idrss:       0,
            ru_isrss:       0,
            ru_minflt:      0,
            ru_majflt:      0,
            ru_nswap:       0,
            ru_inblock:     0,
            ru_oublock:     0,
            ru_msgsnd:      0,
            ru_msgrcv:      0,
            ru_nsignals:    0,
            ru_nvcsw:       0,
            ru_nivcsw:      0,
        };
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut rusage) } != 0 {
            panic!("Failed to retrieve resource usages.")
        }

        Self{
            user_time: duration_from_timeval(rusage.ru_utime),
            system_time: duration_from_timeval(rusage.ru_stime),
            allocated: ALLOCATOR.allocated(),
        }
    }

    /// Compare this [Snapshot] to an earlier one
    pub fn diff(&self, older: Self) -> Diff {
        Diff{
            user_time: self.user_time - older.user_time,
            system_time: self.system_time - older.system_time,
            allocated: self.allocated.saturating_sub(older.allocated)
        }
    }
}


/// The difference between two [Snapshot]s
#[derive(Copy, Clone, Debug)]
struct Diff {
    pub user_time: Duration,
    pub system_time: Duration,
    pub allocated: usize,
}


/// Convert a [libc::timeval] to an [std::time::Duration]
fn duration_from_timeval(val: libc::timeval) -> Duration {
    Duration::new(
        val.tv_sec.try_into().expect("Timeval has unsuitable seconds."),
        (val.tv_usec * 1000).try_into().expect("Timeval has unsuitable microseconds."),
    )
}


struct AccountingAlloc {
    inner: std::alloc::System,
    allocated: atomic::AtomicUsize,
}

impl AccountingAlloc {
    /// Retrieve the number of bytes currently allocated via this allocator.
    fn allocated(&self) -> usize {
        self.allocated.load(atomic::Ordering::SeqCst)
    }
}

unsafe impl std::alloc::GlobalAlloc for AccountingAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.allocated.fetch_add(layout.size(), atomic::Ordering::Release);
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        self.allocated.fetch_sub(layout.size(), atomic::Ordering::Release);
        self.inner.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        if let Some(diff) = layout.size().checked_sub(new_size) {
            self.allocated.fetch_sub(diff, atomic::Ordering::Release);
        } else {
            self.allocated.fetch_add(new_size.saturating_sub(layout.size()), atomic::Ordering::Release);
        }
        self.inner.realloc(ptr, layout, new_size)
    }
}


#[global_allocator]
static ALLOCATOR: AccountingAlloc = AccountingAlloc{inner: std::alloc::System, allocated: atomic::AtomicUsize::new(0)};

