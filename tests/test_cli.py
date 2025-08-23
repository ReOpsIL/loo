#!/usr/bin/env python3
"""
Simple test script to validate Break CLI functionality.
This simulates an LLM sending instructions to the CLI.
"""

import json
import subprocess
import sys
import time
import os
from pathlib import Path

def test_break_cli():
    """Test the Break CLI with a series of LLM-like instructions."""
    
    # Create a test directory
    test_dir = Path("/tmp/break_cli_test")
    test_dir.mkdir(exist_ok=True)
    
    print(f"Testing Break CLI in directory: {test_dir}")
    
    # Start the CLI process
    proc = subprocess.Popen(
        ["cargo", "run", "--bin", "loo", "--", "start", "Test project for CLI validation", "--dir", str(test_dir)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0
    )
    
    # Read the initial query
    try:
        initial_line = proc.stdout.readline()
        if initial_line:
            initial_query = json.loads(initial_line.strip())
            print("✓ Received initial query:")
            print(f"  Project: {initial_query.get('project_description')}")
            print(f"  Working dir: {initial_query.get('working_directory')}")
            print()
        
        # Test sequence of instructions
        test_instructions = [
            {
                "task_id": "test-1",
                "task_type": "create_file",
                "params": {
                    "path": "hello.txt",
                    "content": "Hello from Break CLI test!"
                }
            },
            {
                "task_id": "test-2", 
                "task_type": "create_directory",
                "params": {
                    "path": "test_subdir"
                }
            },
            {
                "task_id": "test-3",
                "task_type": "create_file",
                "params": {
                    "path": "test_subdir/nested.txt",
                    "content": "Nested file content"
                }
            },
            {
                "task_id": "test-4",
                "task_type": "list_directory",
                "params": {
                    "path": "."
                }
            },
            {
                "task_id": "test-5",
                "task_type": "read_file", 
                "params": {
                    "path": "hello.txt"
                }
            },
            {
                "task_id": "test-6",
                "task_type": "run_command",
                "params": {
                    "command": "echo 'Command execution test'"
                }
            },
            {
                "task_id": "test-7",
                "task_type": "query_context",
                "params": {
                    "type": "full"
                }
            },
            {
                "task_id": "test-complete",
                "task_type": "complete",
                "params": {}
            }
        ]
        
        for instruction in test_instructions:
            print(f"→ Sending: {instruction['task_type']} (ID: {instruction['task_id']})")
            
            # Send instruction
            proc.stdin.write(json.dumps(instruction) + "\n")
            proc.stdin.flush()
            
            # Read response
            response_line = proc.stdout.readline()
            if response_line:
                try:
                    feedback = json.loads(response_line.strip())
                    status = feedback.get('status', 'unknown')
                    task_id = feedback.get('task_id', 'unknown')
                    errors = feedback.get('errors', [])
                    
                    if status == 'success':
                        print(f"  ✓ {task_id}: Success")
                    elif status == 'warning':
                        print(f"  ⚠ {task_id}: Warning - {errors}")
                    else:
                        print(f"  ✗ {task_id}: Error - {errors}")
                        
                    # Show some output details for specific tasks
                    if instruction['task_type'] in ['list_directory', 'read_file', 'query_context']:
                        output = feedback.get('output', {})
                        if instruction['task_type'] == 'list_directory':
                            entries = output.get('entries', [])
                            print(f"    Found {len(entries)} entries")
                        elif instruction['task_type'] == 'read_file':
                            content_length = output.get('content_length', 0)
                            print(f"    Read {content_length} characters")
                        elif instruction['task_type'] == 'query_context':
                            dir_listing = output.get('directory_listing', [])
                            print(f"    Context: {len(dir_listing)} files in directory")
                            
                except json.JSONDecodeError as e:
                    print(f"  ✗ Failed to parse response: {e}")
                    print(f"    Raw response: {response_line.strip()}")
            else:
                print("  ✗ No response received")
            
            print()
            
            # Break if we sent the complete instruction
            if instruction['task_type'] == 'complete':
                loo
        
        # Wait for process to finish
        proc.wait(timeout=5)
        print("✓ CLI session completed successfully")
        
    except subprocess.TimeoutExpired:
        print("✗ Process timeout")
        proc.kill()
        return False
    except Exception as e:
        print(f"✗ Test failed: {e}")
        proc.kill()
        return False
    finally:
        # Clean up test directory
        import shutil
        if test_dir.exists():
            shutil.rmtree(test_dir)
            print(f"✓ Cleaned up test directory: {test_dir}")
    
    return True

if __name__ == "__main__":
    print("Break CLI Test Suite")
    print("=" * 50)
    
    if test_break_cli():
        print("\n🎉 All tests passed!")
        sys.exit(0)
    else:
        print("\n❌ Tests failed!")
        sys.exit(1)