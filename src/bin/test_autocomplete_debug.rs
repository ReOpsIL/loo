
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
        println!("\n=== DEBUG INFO ===");
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
        println!("================\n");
        
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
    println!("Type 'quit' to exit\n");
    
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
