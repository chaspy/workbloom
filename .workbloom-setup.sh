#!/bin/bash

# Test setup script for workbloom
echo "🎉 Running .workbloom-setup.sh for worktree setup!"
echo "Current directory: $(pwd)"
echo "Branch: $(git branch --show-current)"

# Create a test marker file to verify the script ran
echo "Setup script executed at $(date)" > .workbloom-setup-marker.txt
echo "✅ Created .workbloom-setup-marker.txt file as proof of execution"

# Example: You could add project-specific tasks here like:
# - Running npm install for specific packages
# - Setting up environment variables
# - Creating necessary directories
# - Any other project-specific initialization

echo "🚀 Project-specific setup completed successfully!"