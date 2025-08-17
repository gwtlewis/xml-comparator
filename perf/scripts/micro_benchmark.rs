#!/usr/bin/env cargo

//! Micro-benchmarks for XML comparison engine
//! 
//! This script runs focused benchmarks on the core XML comparison logic
//! to identify performance bottlenecks at the component level.
//!
//! Usage: cargo run --release micro_benchmark.rs

use std::time::{Duration, Instant};
use std::collections::HashMap;

// Mock XML comparison service (simplified version)
struct XmlComparisonService;

impl XmlComparisonService {
    fn new() -> Self {
        Self
    }
    
    fn compare_xmls(&self, xml1: &str, xml2: &str) -> ComparisonResult {
        let elements1 = self.parse_xml(xml1);
        let elements2 = self.parse_xml(xml2);
        
        let mut diffs = 0;
        let total = elements1.len().max(elements2.len());
        
        // Simplified comparison logic
        for (path, element1) in &elements1 {
            if let Some(element2) = elements2.get(path) {
                if element1 != element2 {
                    diffs += 1;
                }
            } else {
                diffs += 1;
            }
        }
        
        ComparisonResult {
            matched: diffs == 0,
            total_elements: total,
            diffs_count: diffs,
        }
    }
    
    fn parse_xml(&self, xml: &str) -> HashMap<String, String> {
        // Simplified XML parsing - just extract tag names and content
        let mut elements = HashMap::new();
        let mut depth = 0;
        let mut current_path = String::new();
        
        // Very basic parsing - count opening/closing tags
        for line in xml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('<') && !trimmed.starts_with("</") {
                if let Some(tag_name) = extract_tag_name(trimmed) {
                    current_path = format!("/{}", tag_name);
                    elements.insert(current_path.clone(), trimmed.to_string());
                    depth += 1;
                }
            }
        }
        
        elements
    }
}

#[derive(Debug)]
struct ComparisonResult {
    matched: bool,
    total_elements: usize,
    diffs_count: usize,
}

fn extract_tag_name(tag_line: &str) -> Option<String> {
    if let Some(start) = tag_line.find('<') {
        if let Some(end) = tag_line[start+1..].find(' ').or_else(|| tag_line[start+1..].find('>')) {
            return Some(tag_line[start+1..start+1+end].to_string());
        }
    }
    None
}

// Benchmark data generators
fn generate_xml_depth_2(seed: u32, prefix: &str) -> String {
    format!(
        r#"<level2 id="{}_2" value="{}">
            <level1 id="{}_1" value="{}">{}_content</level1>
        </level2>"#,
        prefix, seed, prefix, seed + 1, prefix
    )
}

fn generate_xml_depth_3(seed: u32, prefix: &str) -> String {
    format!(
        r#"<level3 id="{}_3" value="{}">
            <level2 id="{}_2" value="{}">
                <level1 id="{}_1" value="{}">{}_content</level1>
            </level2>
        </level3>"#,
        prefix, seed, prefix, seed + 1, prefix, seed + 2, prefix
    )
}

fn generate_xml_depth_5(seed: u32, prefix: &str) -> String {
    format!(
        r#"<level5 id="{}_5" value="{}">
            <level4 id="{}_4" value="{}">
                <level3 id="{}_3" value="{}">
                    <level2 id="{}_2" value="{}">
                        <level1 id="{}_1" value="{}">{}_content</level1>
                    </level2>
                </level3>
            </level4>
        </level5>"#,
        prefix, seed, prefix, seed + 1, prefix, seed + 2, prefix, seed + 3, prefix, seed + 4, prefix
    )
}

// Benchmark runner
struct BenchmarkRunner {
    service: XmlComparisonService,
    results: Vec<BenchmarkResult>,
}

#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    iterations: usize,
    total_duration: Duration,
    avg_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
    throughput_per_sec: f64,
}

impl BenchmarkRunner {
    fn new() -> Self {
        Self {
            service: XmlComparisonService::new(),
            results: Vec::new(),
        }
    }
    
    fn benchmark<F>(&mut self, name: &str, iterations: usize, mut operation: F)
    where
        F: FnMut() -> (),
    {
        println!("Running benchmark: {} ({} iterations)", name, iterations);
        
        // Warmup
        for _ in 0..10 {
            operation();
        }
        
        let mut durations = Vec::with_capacity(iterations);
        let start_total = Instant::now();
        
        for i in 0..iterations {
            let start = Instant::now();
            operation();
            let duration = start.elapsed();
            durations.push(duration);
            
            if (i + 1) % (iterations / 10).max(1) == 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        }
        
        let total_duration = start_total.elapsed();
        println!(" done");
        
        durations.sort();
        let avg_duration = total_duration / iterations as u32;
        let min_duration = durations[0];
        let max_duration = durations[iterations - 1];
        let throughput_per_sec = iterations as f64 / total_duration.as_secs_f64();
        
        let result = BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_duration,
            avg_duration,
            min_duration,
            max_duration,
            throughput_per_sec,
        };
        
        self.results.push(result);
    }
    
    fn run_all_benchmarks(&mut self) {
        println!("=== XML Comparison Micro-Benchmarks ===\n");
        
        // Single comparison benchmarks
        self.benchmark_single_comparisons();
        
        // Batch comparison benchmarks  
        self.benchmark_batch_comparisons();
        
        // Memory allocation benchmarks
        self.benchmark_memory_patterns();
        
        // Generate report
        self.print_results();
    }
    
    fn benchmark_single_comparisons(&mut self) {
        println!("--- Single Comparison Benchmarks ---");
        
        // Depth 2 identical
        let xml_d2_1 = generate_xml_depth_2(123, "doc1");
        let xml_d2_2 = xml_d2_1.clone();
        self.benchmark("Depth 2 - Identical", 10000, || {
            self.service.compare_xmls(&xml_d2_1, &xml_d2_2);
        });
        
        // Depth 2 different
        let xml_d2_3 = generate_xml_depth_2(456, "doc2");
        self.benchmark("Depth 2 - Different", 10000, || {
            self.service.compare_xmls(&xml_d2_1, &xml_d2_3);
        });
        
        // Depth 3 identical
        let xml_d3_1 = generate_xml_depth_3(123, "doc1");
        let xml_d3_2 = xml_d3_1.clone();
        self.benchmark("Depth 3 - Identical", 5000, || {
            self.service.compare_xmls(&xml_d3_1, &xml_d3_2);
        });
        
        // Depth 3 different
        let xml_d3_3 = generate_xml_depth_3(456, "doc2");
        self.benchmark("Depth 3 - Different", 5000, || {
            self.service.compare_xmls(&xml_d3_1, &xml_d3_3);
        });
        
        // Depth 5 identical
        let xml_d5_1 = generate_xml_depth_5(123, "doc1");
        let xml_d5_2 = xml_d5_1.clone();
        self.benchmark("Depth 5 - Identical", 1000, || {
            self.service.compare_xmls(&xml_d5_1, &xml_d5_2);
        });
        
        // Depth 5 different
        let xml_d5_3 = generate_xml_depth_5(456, "doc2");
        self.benchmark("Depth 5 - Different", 1000, || {
            self.service.compare_xmls(&xml_d5_1, &xml_d5_3);
        });
    }
    
    fn benchmark_batch_comparisons(&mut self) {
        println!("\n--- Batch Comparison Benchmarks ---");
        
        // Generate test data
        let mut xmls_d2: Vec<(String, String)> = Vec::new();
        let mut xmls_d3: Vec<(String, String)> = Vec::new();
        
        for i in 0..1000 {
            let xml1 = generate_xml_depth_2(i, &format!("batch{}", i));
            let xml2 = if i % 3 == 0 {
                generate_xml_depth_2(i + 1000, &format!("batch{}", i)) // Different
            } else {
                xml1.clone() // Same
            };
            xmls_d2.push((xml1, xml2));
            
            let xml1 = generate_xml_depth_3(i, &format!("batch{}", i));
            let xml2 = if i % 3 == 0 {
                generate_xml_depth_3(i + 1000, &format!("batch{}", i)) // Different
            } else {
                xml1.clone() // Same
            };
            xmls_d3.push((xml1, xml2));
        }
        
        // Batch depth 2
        self.benchmark("Batch 1000 - Depth 2", 10, || {
            for (xml1, xml2) in &xmls_d2 {
                self.service.compare_xmls(xml1, xml2);
            }
        });
        
        // Batch depth 3
        self.benchmark("Batch 1000 - Depth 3", 5, || {
            for (xml1, xml2) in &xmls_d3 {
                self.service.compare_xmls(xml1, xml2);
            }
        });
    }
    
    fn benchmark_memory_patterns(&mut self) {
        println!("\n--- Memory Pattern Benchmarks ---");
        
        // Large XML documents
        let large_xml_1 = generate_large_xml(100, "large1");
        let large_xml_2 = generate_large_xml(100, "large2");
        
        self.benchmark("Large XML (100 elements)", 100, || {
            self.service.compare_xmls(&large_xml_1, &large_xml_2);
        });
        
        // Very large XML documents
        let very_large_xml_1 = generate_large_xml(1000, "xlarge1");
        let very_large_xml_2 = generate_large_xml(1000, "xlarge2");
        
        self.benchmark("Very Large XML (1000 elements)", 10, || {
            self.service.compare_xmls(&very_large_xml_1, &very_large_xml_2);
        });
    }
    
    fn print_results(&self) {
        println!("\n=== Benchmark Results ===");
        println!("{:<30} {:>10} {:>15} {:>15} {:>15} {:>15}", 
                "Benchmark", "Iterations", "Avg (ms)", "Min (ms)", "Max (ms)", "Ops/sec");
        println!("{}", "-".repeat(100));
        
        for result in &self.results {
            println!("{:<30} {:>10} {:>15.2} {:>15.2} {:>15.2} {:>15.0}",
                result.name,
                result.iterations,
                result.avg_duration.as_millis() as f64,
                result.min_duration.as_millis() as f64,
                result.max_duration.as_millis() as f64,
                result.throughput_per_sec
            );
        }
        
        println!("\n=== Performance Analysis ===");
        
        // Find fastest and slowest operations
        if let (Some(fastest), Some(slowest)) = (
            self.results.iter().max_by(|a, b| a.throughput_per_sec.partial_cmp(&b.throughput_per_sec).unwrap()),
            self.results.iter().min_by(|a, b| a.throughput_per_sec.partial_cmp(&b.throughput_per_sec).unwrap())
        ) {
            println!("Fastest operation: {} ({:.0} ops/sec)", fastest.name, fastest.throughput_per_sec);
            println!("Slowest operation: {} ({:.0} ops/sec)", slowest.name, slowest.throughput_per_sec);
            
            let ratio = fastest.throughput_per_sec / slowest.throughput_per_sec;
            println!("Performance ratio: {:.1}x", ratio);
        }
        
        // Calculate 100k projection
        if let Some(depth2_result) = self.results.iter().find(|r| r.name.contains("Depth 2 - Different")) {
            let time_for_100k = Duration::from_secs_f64(100_000.0 / depth2_result.throughput_per_sec);
            println!("\nProjected time for 100k depth-2 comparisons: {:.1}s", time_for_100k.as_secs_f64());
        }
    }
}

fn generate_large_xml(element_count: usize, prefix: &str) -> String {
    let mut xml = format!("<root id=\"{}\">\n", prefix);
    
    for i in 0..element_count {
        xml.push_str(&format!(
            "  <item{} id=\"{}{}\" value=\"{}\" type=\"test\">{}_content_{}</item{}>\n",
            i, prefix, i, i * 7, prefix, i, i
        ));
    }
    
    xml.push_str("</root>");
    xml
}

fn main() {
    let mut runner = BenchmarkRunner::new();
    runner.run_all_benchmarks();
    
    println!("\n=== Recommendations ===");
    println!("1. Monitor depth-5 performance closely in production");
    println!("2. Consider streaming/chunking for very large XMLs");
    println!("3. Profile memory allocations if processing >10k pairs/batch");
    println!("4. Implement caching for repeated identical comparisons");
}

// Cargo.toml inline dependencies
/*
[dependencies]
# No external dependencies needed for this benchmark
*/
