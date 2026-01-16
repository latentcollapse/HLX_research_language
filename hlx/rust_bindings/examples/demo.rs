use test_ffi_lib::{add, multiply};

fn main() {
    println!("HLX Rust FFI Demo\n");

    // Basic operations
    let sum = add(100, 200);
    println!("add(100, 200) = {}", sum);

    let product = multiply(7, 8);
    println!("multiply(7, 8) = {}", product);

    // Function composition
    let x = add(50, 50);
    let y = multiply(x, 3);
    println!("multiply(add(50, 50), 3) = {}", y);

    // Performance test
    let iterations = 100_000;
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let _ = add(i, i + 1);
        let _ = multiply(i, 2);
    }
    let elapsed = start.elapsed();
    let calls_per_sec = (iterations * 2) as f64 / elapsed.as_secs_f64();

    println!("\nPerformance Test ({} iterations):", iterations);
    println!("  Time: {:.4}s ({:.0} calls/sec)", elapsed.as_secs_f64(), calls_per_sec);
    println!("\nSuccess!");
}
