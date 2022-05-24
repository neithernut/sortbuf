//! Comparison between different kinds of sorting buffers


fn main() {
    // TODO: impl
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

