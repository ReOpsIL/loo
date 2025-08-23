#!/bin/bash
echo "Testing final text wrapping implementation"
echo "Try typing a very long line to see the wrapping in action"
echo "Use Ctrl+C three times to exit"
echo ""
echo "Long text test:"
echo "The quick brown fox jumps over the lazy dog. This sentence should wrap naturally when it exceeds the terminal width, showing proper multi-line display behavior."
echo ""
./target/debug/loo start "test wrapping"