static mut ALLOCATIONS: Vec<Allocation> = Vec::new();

/// An allocation to a value of size `size`.
struct Allocation {
    size: usize,
    ptr: *mut (),
}
