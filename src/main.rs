use std::{hint::black_box, time::Instant};
use warded_path_constraint_probe::{Approach, Routes, request};

#[cfg(feature = "count-allocs")]
mod allocation_count {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct CountingAllocator;
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static BYTES: AtomicUsize = AtomicUsize::new(0);

    // Instrumentation is confined to a separate build, never used for timing results.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            // SAFETY: forward the unchanged allocator contract to System.
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            // SAFETY: forward the unchanged allocator contract to System.
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            // SAFETY: all allocations are delegated to System with this layout.
            unsafe { System.dealloc(pointer, layout) }
        }
        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
            // SAFETY: forward the unchanged reallocation contract to System.
            unsafe { System.realloc(pointer, layout, size) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    pub fn measure(mut operation: impl FnMut(), iterations: usize) -> (f64, f64) {
        CALLS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        for _ in 0..iterations {
            operation();
        }
        (
            CALLS.load(Ordering::Relaxed) as f64 / iterations as f64,
            BYTES.load(Ordering::Relaxed) as f64 / iterations as f64,
        )
    }
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "bench".into());
    let iterations = std::env::var("PROBE_ITERS")
        .unwrap_or_else(|_| "100000".into())
        .parse::<usize>()
        .unwrap();
    assert!(iterations > 0);
    let mut special = request("/users/new");
    special
        .headers_mut()
        .insert("x-special", "yes".parse().unwrap());
    let cases = [
        ("static_hit", special),
        ("short_id", request("/users/42")),
        ("maximum_id", request("/users/18446744073709551615")),
        ("username_fallback", request("/users/Alice_42")),
        ("same_path_header_miss", request("/users/new")),
        ("overflow_miss", request("/users/18446744073709551616")),
        ("invalid_miss", request("/users/a-b")),
        ("unrelated_path", request("/orders/42")),
    ];
    let approaches = Approach::all();
    let routes = approaches
        .iter()
        .map(|&approach| Routes::new(approach))
        .collect::<Vec<_>>();
    // Every measured input must select the same route, raw values and handler result.
    for (_, request) in &cases {
        let reference = routes[0].execute(request);
        for route in &routes {
            assert_eq!(route.execute(request), reference);
        }
    }
    for route in &routes {
        for (_, request) in &cases {
            for _ in 0..5000 {
                black_box(route.execute(black_box(request)));
            }
        }
    }
    if mode == "alloc" {
        #[cfg(feature = "count-allocs")]
        {
            println!("scenario,approach,allocations_per_operation,requested_bytes_per_operation");
            for (scenario, request) in &cases {
                for (index, route) in routes.iter().enumerate() {
                    let (calls, bytes) = allocation_count::measure(
                        || {
                            black_box(route.execute(black_box(request)));
                        },
                        10_000,
                    );
                    println!(
                        "{scenario},{},{calls:.4},{bytes:.4}",
                        approaches[index].name()
                    );
                }
            }
            return;
        }
        #[cfg(not(feature = "count-allocs"))]
        panic!("allocation measurements require --features count-allocs");
    }
    assert_eq!(
        (mode.as_str(), cfg!(feature = "count-allocs")),
        ("bench", false),
        "timing requires bench mode without the counting allocator"
    );
    println!("kind,scenario,approach,median_ns,min_ns,max_ns");
    for (scenario, request) in &cases {
        let mut samples = vec![Vec::new(); routes.len()];
        for round in 0..9 {
            // Rotate the order each round to reduce systematic ordering bias.
            for offset in 0..routes.len() {
                let index = (round + offset) % routes.len();
                let start = Instant::now();
                for _ in 0..iterations {
                    black_box(routes[index].execute(black_box(request)));
                }
                samples[index].push(start.elapsed().as_nanos() as f64 / iterations as f64);
            }
        }
        for (index, samples) in samples.iter_mut().enumerate() {
            report(
                "dispatch_and_handler",
                scenario,
                approaches[index].name(),
                samples,
            );
        }
    }
    let mut samples = vec![Vec::new(); approaches.len()];
    for round in 0..9 {
        for offset in 0..approaches.len() {
            let index = (round + offset) % approaches.len();
            let start = Instant::now();
            for _ in 0..100 {
                black_box(Routes::new(black_box(approaches[index])));
            }
            samples[index].push(start.elapsed().as_nanos() as f64 / 100.0);
        }
    }
    for (index, samples) in samples.iter_mut().enumerate() {
        report(
            "table_build_and_drop",
            "three_routes",
            approaches[index].name(),
            samples,
        );
    }
}

fn report(kind: &str, scenario: &str, approach: &str, samples: &mut [f64]) {
    samples.sort_by(f64::total_cmp);
    println!(
        "{kind},{scenario},{approach},{:.3},{:.3},{:.3}",
        samples[samples.len() / 2],
        samples[0],
        samples[samples.len() - 1]
    );
}
