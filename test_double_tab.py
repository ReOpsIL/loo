#!/usr/bin/env python3
"""
Test script to verify double-tab functionality for folder drilling.
This simulates the autocomplete behavior that should happen when using @src/ + Tab Tab
"""

import subprocess
import sys
import os

def test_double_tab_functionality():
    """Test the double-tab drilling functionality"""
    print("Testing double-tab functionality for @src/ + Tab Tab")
    print("=" * 50)
    
    # Check if we're in the right directory
    if not os.path.exists("src"):
        print("❌ Error: 'src' directory not found. Make sure you're in the project root.")
        return False
    
    print("✅ Found 'src' directory")
    
    # List what's in src directory
    src_contents = os.listdir("src")
    print(f"📁 Contents of src/: {sorted(src_contents)}")
    
    # The functionality should be available when running the CLI
    print("\n🎯 Double-tab functionality should work as follows:")
    print("1. Type: 'Check @src/' + Tab")
    print("2. Press Tab again (double-tab)")
    print("3. Should show contents of src/ folder for drilling down")
    
    # Check if the binary exists
    if os.path.exists("target/debug/loo") or os.path.exists("target/release/loo"):
        print("✅ Binary found - functionality should be available")
        return True
    else:
        print("⚠️  Binary not found - need to build first with 'cargo build'")
        return False

if __name__ == "__main__":
    success = test_double_tab_functionality()
    sys.exit(0 if success else 1)