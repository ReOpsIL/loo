#!/usr/bin/env python3
"""
Debug script to test the autocomplete behavior and understand why double-tab might not work
"""

import subprocess
import os
import sys

def create_test_binary():
    """Create a simple test binary to debug autocomplete behavior"""
    test_code = '''
use inquire::{Text, Autocomplete};
use std::path::Path;
use std::fs;

#[derive(Clone)]
struct DebugAutocomplete {
    tab_count: u32,
    last_input: String,
}

impl DebugAutocomplete {
    fn new() -> Self {
        Self {
            tab_count: 0,
            last_input: String::new(),
        }
    }
}

impl Autocomplete for DebugAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        println!("\\n=== DEBUG INFO ===");
        println!("Current input: '{}'", input);
        println!("Last input: '{}'", self.last_input);
        println!("Current tab count: {}", self.tab_count);
        
        let is_repeat_tab = input == self.last_input;
        if is_repeat_tab {
            self.tab_count += 1;
        } else {
            self.tab_count = 1;
        }
        self.last_input = input.to_string();
        
        println!("Is repeat tab: {}", is_repeat_tab);
        println!("New tab count: {}", self.tab_count);
        println!("================\\n");
        
        if input.contains("@") && input.ends_with("src/") && self.tab_count >= 2 {
            println!("🎯 DOUBLE-TAB DETECTED ON FOLDER!");
            return Ok(vec![
                format!("{}bin/", input),
                format!("{}commands/", input),
                format!("{}engine.rs", input),
            ]);
        }
        
        if input.contains("@") {
            return Ok(vec![
                format!("{}src/", input.split('@').next().unwrap_or("") + "@"),
                format!("{}target/", input.split('@').next().unwrap_or("") + "@"),
            ]);
        }
        
        Ok(vec![])
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(match highlighted_suggestion {
            Some(suggestion) => inquire::autocompletion::Replacement::Some(suggestion),
            None => inquire::autocompletion::Replacement::None,
        })
    }
}

fn main() {
    println!("🔍 Testing autocomplete behavior");
    println!("Try typing '@src/' and press Tab twice");
    println!("Type 'quit' to exit\\n");
    
    let autocomplete = DebugAutocomplete::new();
    
    loop {
        let result = Text::new("Test input:")
            .with_autocomplete(autocomplete.clone())
            .prompt();
            
        match result {
            Ok(input) => {
                if input == "quit" {
                    break;
                }
                println!("You entered: {}", input);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}
'''
    
    # Write the test code
    with open("src/bin/test_autocomplete_debug.rs", "w") as f:
        f.write(test_code)
    
    return "src/bin/test_autocomplete_debug.rs"

def main():
    print("Creating debug test binary...")
    test_file = create_test_binary()
    
    print(f"Created {test_file}")
    print("Building...")
    
    # Build the test binary
    result = subprocess.run(["cargo", "build", "--bin", "test_autocomplete_debug"], 
                          capture_output=True, text=True)
    
    if result.returncode != 0:
        print("❌ Build failed:")
        print(result.stderr)
        return False
    
    print("✅ Build successful!")
    print("Run with: ./target/debug/test_autocomplete_debug")
    print("Then try typing '@src/' and pressing Tab twice")
    
    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)