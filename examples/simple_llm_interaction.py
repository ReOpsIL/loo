#!/usr/bin/env python3
"""
Example of how an LLM would interact with the Break CLI.
This demonstrates the complete workflow from project initiation to completion.
"""

import json
import subprocess
import sys
from pathlib import Path

class BreakCLIInterface:
    """Simple interface to interact with Break CLI as an LLM would."""
    
    def __init__(self, project_description, working_dir=None):
        self.project_description = project_description
        self.working_dir = working_dir or "/tmp/break_example"
        self.task_counter = 0
        
        # Ensure working directory exists
        Path(self.working_dir).mkdir(parents=True, exist_ok=True)
        
        # Start CLI process
        self.proc = subprocess.Popen(
            ["cargo", "run", "--bin", "loo", "--", "start", project_description, "--dir", self.working_dir],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0,
            cwd=Path(__file__).parent.parent  # Run from loo project root
        )
        
        # Read initial query
        self.initial_query = self._read_response()
        print(f"🤖 LLM: Starting project: {project_description}")
        print(f"    Working in: {self.initial_query.get('working_directory', 'unknown')}")
        print()
    
    def _read_response(self):
        """Read a JSON response from the CLI."""
        line = self.proc.stdout.readline()
        if line:
            return json.loads(line.strip())
        return None
    
    def _send_instruction(self, task_type, params=None, follow_up_query=None):
        """Send an instruction to the CLI and return the feedback."""
        self.task_counter += 1
        instruction = {
            "task_id": f"task-{self.task_counter}",
            "task_type": task_type,
            "params": params or {},
        }
        
        if follow_up_query:
            instruction["follow_up_query"] = follow_up_query
            
        print(f"🤖 LLM: Sending task '{task_type}' (ID: {instruction['task_id']})")
        
        # Send instruction
        self.proc.stdin.write(json.dumps(instruction) + "\n")
        self.proc.stdin.flush()
        
        # Read feedback
        feedback = self._read_response()
        if feedback:
            status = feedback.get('status', 'unknown')
            if status == 'success':
                print(f"✅ CLI: Task completed successfully")
            elif status == 'warning':
                print(f"⚠️  CLI: Task completed with warnings: {feedback.get('errors', [])}")
            else:
                print(f"❌ CLI: Task failed: {feedback.get('errors', [])}")
        
        return feedback
    
    def simulate_rust_hello_world_project(self):
        """Simulate an LLM creating a simple Rust Hello World project."""
        
        print("🤖 LLM: I'll create a simple Rust Hello World project.")
        print("    Let me start by creating the basic project structure.")
        print()
        
        # Step 1: Create Cargo.toml
        cargo_toml_content = '''[package]
name = "hello-world"
version = "0.1.0"
edition = "2021"

[dependencies]
'''
        
        self._send_instruction("create_file", {
            "path": "Cargo.toml",
            "content": cargo_toml_content
        })
        
        print("🤖 LLM: Created Cargo.toml. Now I'll create the source directory.")
        print()
        
        # Step 2: Create src directory
        self._send_instruction("create_directory", {"path": "src"})
        
        print("🤖 LLM: Created src directory. Now I'll add the main.rs file.")
        print()
        
        # Step 3: Create main.rs
        main_rs_content = '''fn main() {
    println!("Hello, world from Break CLI!");
    println!("This project was created by an LLM using the Break CLI.");
}
'''
        
        self._send_instruction("create_file", {
            "path": "src/main.rs", 
            "content": main_rs_content
        })
        
        print("🤖 LLM: Created main.rs. Let me verify the project structure.")
        print()
        
        # Step 4: List directory to verify structure
        feedback = self._send_instruction("list_directory", {"path": "."})
        if feedback and feedback.get('status') == 'success':
            entries = feedback.get('output', {}).get('entries', [])
            print(f"    📁 Found {len(entries)} items in project root")
            for entry in entries:
                icon = "📁" if entry.get('is_dir') else "📄"
                print(f"    {icon} {entry.get('name')}")
        print()
        
        print("🤖 LLM: Now I'll test if the project builds correctly.")
        print()
        
        # Step 5: Build the project
        feedback = self._send_instruction("run_command", {
            "command": "cargo check"
        })
        
        if feedback and feedback.get('status') == 'success':
            output = feedback.get('output', {})
            if output.get('success'):
                print("    ✅ Project builds successfully!")
            else:
                print(f"    ❌ Build failed: {output.get('stderr', 'Unknown error')}")
        print()
        
        print("🤖 LLM: Let me also test running the program.")
        print()
        
        # Step 6: Run the program
        feedback = self._send_instruction("run_command", {
            "command": "cargo run"
        })
        
        if feedback and feedback.get('status') == 'success':
            output = feedback.get('output', {})
            if output.get('success'):
                stdout = output.get('stdout', '').strip()
                print("    🎯 Program output:")
                for line in stdout.split('\n'):
                    if line.strip():
                        print(f"       {line}")
            else:
                print(f"    ❌ Run failed: {output.get('stderr', 'Unknown error')}")
        print()
        
        print("🤖 LLM: Perfect! The project is working. Let me also create a README.")
        print()
        
        # Step 7: Create README.md
        readme_content = '''# Hello World Rust Project

This is a simple Hello World project in Rust, created by an LLM using the Break CLI.

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

## About

This project demonstrates:
- Basic Rust project structure
- Cargo.toml configuration
- Simple console output
- LLM-driven development workflow using Break CLI
'''
        
        self._send_instruction("create_file", {
            "path": "README.md",
            "content": readme_content
        })
        
        print("🤖 LLM: Excellent! The project is complete. Let me get a final context overview.")
        print()
        
        # Step 8: Final context query
        feedback = self._send_instruction("query_context", {"type": "full"})
        if feedback and feedback.get('status') == 'success':
            context = feedback.get('output', {})
            files = context.get('directory_listing', [])
            print(f"    📊 Final project contains {len(files)} files:")
            for file in files:
                print(f"       📄 {file}")
        print()
        
        # Step 9: Mark project as complete
        print("🤖 LLM: Project completed successfully! Marking as complete.")
        print()
        
        self._send_instruction("complete", {})
        
        print("🎉 LLM: All done! The Rust Hello World project is ready to use.")
        print()
    
    def cleanup(self):
        """Clean up the CLI process."""
        try:
            self.proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        
        # Clean up working directory
        import shutil
        if Path(self.working_dir).exists():
            shutil.rmtree(self.working_dir)
            print(f"🧹 Cleaned up working directory: {self.working_dir}")

def main():
    print("Break CLI - LLM Interaction Example")
    print("=" * 50)
    print("This demonstrates how an LLM would use the Break CLI")
    print("to create a complete Rust project from scratch.")
    print()
    
    # Create interface and run simulation
    interface = BreakCLIInterface("Create a simple Rust Hello World project with proper structure and documentation")
    
    try:
        interface.simulate_rust_hello_world_project()
    except Exception as e:
        print(f"❌ Error during simulation: {e}")
        return False
    finally:
        interface.cleanup()
    
    print("✨ Simulation completed successfully!")
    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)