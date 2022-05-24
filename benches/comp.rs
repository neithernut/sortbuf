//! Comparison between different kinds of sorting buffers


fn main() {
    // TODO: impl
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

