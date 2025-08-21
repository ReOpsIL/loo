#!/bin/bash

echo "Testing recursive execution system..."

# Start loo with a test session
./target/debug/loo start "test session" << 'EOF'
/plan Create a simple web project with three components: 1) Create index.html with basic structure, 2) Create style.css with styling, 3) Create main.js with interactive functionality
EOF