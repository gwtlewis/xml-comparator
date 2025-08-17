#!/usr/bin/env cargo

//! XML Payload Generator for Performance Testing
//! 
//! Generates 100k XML pairs with specified depth distribution:
//! - 10% depth 5 (10,000 pairs)
//! - 30% depth 3 (30,000 pairs) 
//! - 60% depth 2 (60,000 pairs)
//!
//! Usage: cargo run --release gen_payload.rs -- <count> [seed]

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde_json::json;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <count> [seed]", args[0]);
        std::process::exit(1);
    }

    let count: usize = args[1].parse().expect("Invalid count");
    let seed: u64 = args.get(2).map(|s| s.parse().expect("Invalid seed")).unwrap_or(42);
    
    let mut rng = StdRng::seed_from_u64(seed);
    
    eprintln!("Generating {} XML pairs with seed {}", count, seed);
    
    let mut comparisons = Vec::with_capacity(count);
    
    for i in 0..count {
        let depth = determine_depth(i, count);
        let base_seed = rng.gen::<u32>();
        
        // Generate xml1
        let xml1 = generate_xml(depth, base_seed, &format!("doc{}", i));
        
        // Generate xml2 - 70% identical, 30% different
        let xml2 = if rng.gen::<f32>() < 0.7 {
            xml1.clone() // Identical
        } else {
            // Generate slightly different XML
            if rng.gen::<bool>() {
                // Change an attribute
                generate_xml_with_different_attr(depth, base_seed, &format!("doc{}", i))
            } else {
                // Change content
                generate_xml_with_different_content(depth, base_seed, &format!("doc{}", i))
            }
        };
        
        comparisons.push(json!({
            "xml1": xml1,
            "xml2": xml2,
            "ignore_paths": [],
            "ignore_properties": []
        }));
        
        if (i + 1) % 10000 == 0 {
            eprintln!("Generated {}/{} pairs", i + 1, count);
        }
    }
    
    let payload = json!({
        "comparisons": comparisons
    });
    
    println!("{}", serde_json::to_string(&payload).expect("Failed to serialize"));
    eprintln!("Payload generation complete");
}

fn determine_depth(index: usize, total: usize) -> u8 {
    let percent = (index as f64 / total as f64) * 100.0;
    if percent < 10.0 {
        5 // First 10% get depth 5
    } else if percent < 40.0 {
        3 // Next 30% get depth 3  
    } else {
        2 // Remaining 60% get depth 2
    }
}

fn generate_xml(depth: u8, seed: u32, prefix: &str) -> String {
    generate_xml_recursive(depth, seed, prefix, false, false)
}

fn generate_xml_with_different_attr(depth: u8, seed: u32, prefix: &str) -> String {
    generate_xml_recursive(depth, seed, prefix, true, false)
}

fn generate_xml_with_different_content(depth: u8, seed: u32, prefix: &str) -> String {
    generate_xml_recursive(depth, seed, prefix, false, true)
}

fn generate_xml_recursive(depth: u8, seed: u32, prefix: &str, alter_attr: bool, alter_content: bool) -> String {
    if depth == 0 {
        let content = if alter_content {
            format!("{}_{}_CHANGED", prefix, seed)
        } else {
            format!("{}_{}", prefix, seed)
        };
        return content;
    }
    
    let tag = format!("level{}", depth);
    let attr_value = if alter_attr && depth == 1 {
        format!("{}_CHANGED", seed)
    } else {
        seed.to_string()
    };
    
    let inner = generate_xml_recursive(depth - 1, seed + 1, prefix, alter_attr, alter_content);
    
    format!(
        "<{tag} id=\"{id}\" value=\"{attr}\">{inner}</{tag}>",
        tag = tag,
        id = format!("{}_{}", prefix, depth),
        attr = attr_value,
        inner = inner
    )
}

// Cargo.toml inline dependencies
/*
[dependencies]
serde_json = "1.0"
rand = "0.8"
*/
