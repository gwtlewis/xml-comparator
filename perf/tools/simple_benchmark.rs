#!/usr/bin/env cargo

//! Simple XML comparison micro-benchmark
//! This is a simplified version for validation that avoids complex borrowing

use std::time::Instant;

fn simple_xml_compare(xml1: &str, xml2: &str) -> bool {
    // Very simple comparison - just check if strings are equal
    xml1 == xml2
}

fn generate_xml(depth: u8, seed: u32, prefix: &str) -> String {
    if depth == 0 {
        return format!("{}_{}", prefix, seed);
    }
    
    let tag = format!("level{}", depth);
    let inner = generate_xml(depth - 1, seed + 1, prefix);
    
    format!(
        "<{tag} id=\"{prefix}_{depth}\" value=\"{seed}\">{inner}</{tag}>",
        tag = tag,
        prefix = prefix,
        depth = depth,
        seed = seed,
        inner = inner
    )
}

fn benchmark_operation<F>(name: &str, iterations: usize, mut operation: F) 
where
    F: FnMut() -> (),
{
    println!("Running {}: {} iterations", name, iterations);
    
    // Warmup
    for _ in 0..10 {
        operation();
    }
    
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    let duration = start.elapsed();
    
    let avg_ms = duration.as_millis() as f64 / iterations as f64;
    let ops_per_sec = iterations as f64 / duration.as_secs_f64();
    
    println!("  Average: {:.3}ms per operation", avg_ms);
    println!("  Throughput: {:.0} operations/second", ops_per_sec);
}

fn main() {
    println!("=== Simple XML Comparison Benchmark ===\n");
    
    // Test data
    let xml_d2_1 = generate_xml(2, 123, "doc1");
    let xml_d2_2 = xml_d2_1.clone();
    let xml_d2_3 = generate_xml(2, 456, "doc2");
    
    let xml_d3_1 = generate_xml(3, 123, "doc1");
    let xml_d3_2 = xml_d3_1.clone();
    
    let xml_d5_1 = generate_xml(5, 123, "doc1");
    let xml_d5_2 = xml_d5_1.clone();
    
    // Benchmarks
    benchmark_operation("Depth 2 - Identical", 10000, || {
        simple_xml_compare(&xml_d2_1, &xml_d2_2);
    });
    
    benchmark_operation("Depth 2 - Different", 10000, || {
        simple_xml_compare(&xml_d2_1, &xml_d2_3);
    });
    
    benchmark_operation("Depth 3 - Identical", 5000, || {
        simple_xml_compare(&xml_d3_1, &xml_d3_2);
    });
    
    benchmark_operation("Depth 5 - Identical", 1000, || {
        simple_xml_compare(&xml_d5_1, &xml_d5_2);
    });
    
    println!("\n=== Summary ===");
    println!("Simple benchmark completed successfully!");
    println!("This validates that the benchmark framework can compile and run.");
}

// Cargo.toml inline dependencies
/*
[dependencies]
# No external dependencies needed
*/
