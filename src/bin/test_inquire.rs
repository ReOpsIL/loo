use inquire::{Text, Confirm};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing inquire text input");
    
    loop {
        let user_input = Text::new("💬 You:")
            .with_help_message("Type your message (or 'quit' to exit)")
            .prompt();

        match user_input {
            Ok(message) => {
                let message = message.trim();
                
                if message.is_empty() {
                    continue;
                }
                
                if message.to_lowercase() == "quit" || message.to_lowercase() == "exit" {
                    break;
                }
                
                if message.starts_with('/') {
                    println!("🔧 Command: {}", &message[1..]);
                } else {
                    println!("💬 You said: {}", message);
                    
                    if message.len() > 50 {
                        println!("📏 That's a long message! ({} characters)", message.len());
                    }
                }
            }
            Err(inquire::InquireError::OperationCanceled) => {
                println!("\n👋 Goodbye!");
                break;
            }
            Err(inquire::InquireError::OperationInterrupted) => {
                println!("\n👋 Goodbye!");
                break;
            }
            Err(e) => {
                println!("❌ Input error: {}", e);
                continue;
            }
        }
    }
    
    // Test long text input
    let should_test_long = Confirm::new("Do you want to test long text input?")
        .with_default(false)
        .prompt();
    
    if let Ok(true) = should_test_long {
        let long_input = Text::new("Enter a very long text:")
            .with_help_message("This will test text wrapping and multiline support")
            .prompt();
            
        match long_input {
            Ok(text) => {
                println!("✅ Successfully captured {} characters:", text.len());
                println!("📝 Content: {}", text);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
    
    println!("✅ Inquire test completed!");
    Ok(())
}