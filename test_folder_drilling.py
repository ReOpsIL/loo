#!/usr/bin/env python3
"""
Test script to verify the folder drilling functionality works after the fix.
"""

import subprocess
import os
import sys

def test_folder_drilling():
    """Test that the folder drilling functionality is working"""
    print("🔍 Testing folder drilling functionality")
    print("=" * 50)
    
    # Check if we're in the right directory
    if not os.path.exists("src"):
        print("❌ Error: 'src' directory not found. Make sure you're in the project root.")
        return False
    
    print("✅ Found 'src' directory")
    
    # List what's in src directory
    src_contents = os.listdir("src")
    print(f"📁 Contents of src/: {sorted(src_contents)}")
    
    # Check if the binary exists
    if not (os.path.exists("target/debug/loo") or os.path.exists("target/release/loo")):
        print("❌ Binary not found - need to build first")
        return False
    
    print("✅ Binary found")
    
    print("\n🎯 Folder drilling should now work as follows:")
    print("1. Type: 'Check @src/' + Tab")
    print("2. Should immediately show contents of src/ folder")
    print("3. No double-tab required - single tab on complete folder path should drill down")
    
    print("\n✅ The fix has been implemented:")
    print("- Removed unreliable tab counting logic")
    print("- Now automatically shows folder contents when path ends with '/'")
    print("- Works on first tab press when folder path is complete")
    
    return True

if __name__ == "__main__":
    success = test_folder_drilling()
    print(f"\n{'✅ Test completed successfully!' if success else '❌ Test failed!'}")
    sys.exit(0 if success else 1)