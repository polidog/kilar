#!/bin/bash
set -e

echo "Setting up Homebrew tap repository for kilar..."
echo ""
echo "This script will help you create a homebrew-tap repository on GitHub."
echo ""

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed."
    echo "Please install it first: brew install gh"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with GitHub."
    echo "Please run: gh auth login"
    exit 1
fi

# Get current directory
CURRENT_DIR=$(pwd)
TEMP_DIR=$(mktemp -d)

echo "Creating homebrew-tap repository on GitHub..."

# Create the repository
gh repo create homebrew-kilar --public --description "Homebrew tap for kilar - A powerful CLI tool for managing port processes" || {
    echo "Repository might already exist or creation failed."
    echo "If it exists, we'll continue with the setup."
}

echo ""
echo "Cloning homebrew-kilar repository..."
cd "$TEMP_DIR"

# Clone the repository
if gh repo clone polidog/homebrew-kilar 2>/dev/null; then
    cd homebrew-kilar
else
    echo "Failed to clone repository. It might not exist yet."
    echo "Creating local repository..."
    mkdir homebrew-kilar
    cd homebrew-kilar
    git init
    git remote add origin https://github.com/polidog/homebrew-kilar.git
fi

echo ""
echo "Setting up tap structure..."

# Copy Formula directory
cp -r "$CURRENT_DIR/Formula" .

# Copy and rename README
if [ -f "$CURRENT_DIR/homebrew-tap-README.md" ]; then
    cp "$CURRENT_DIR/homebrew-tap-README.md" README.md
else
    echo "# Homebrew Tap for kilar" > README.md
    echo "" >> README.md
    echo "This repository contains the Homebrew formula for [kilar](https://github.com/polidog/kilar)." >> README.md
    echo "" >> README.md
    echo "## Installation" >> README.md
    echo "" >> README.md
    echo '```bash' >> README.md
    echo "brew tap polidog/kilar" >> README.md
    echo "brew install kilar" >> README.md
    echo '```' >> README.md
fi

# Create .gitignore
cat > .gitignore << 'EOF'
.DS_Store
*.swp
*.swo
*~
EOF

echo ""
echo "Committing and pushing to GitHub..."

git add .
git commit -m "Initial setup of Homebrew tap for kilar" || {
    echo "Nothing to commit or commit failed."
}

# Push to GitHub
git branch -M main
git push -u origin main || {
    echo "Push failed. You may need to set up the repository manually."
    echo "Repository location: $TEMP_DIR/homebrew-kilar"
}

echo ""
echo "✅ Homebrew tap repository setup complete!"
echo ""
echo "Repository: https://github.com/polidog/homebrew-kilar"
echo ""
echo "Users can now install kilar with:"
echo "  brew tap polidog/kilar"
echo "  brew install kilar"
echo ""
echo "Or directly:"
echo "  brew install polidog/kilar/kilar"
echo ""
echo "Note: You'll need to update the SHA256 checksums in Formula/kilar.rb"
echo "after creating a release. Use: ./scripts/update-formula.sh <version>"

# Return to original directory
cd "$CURRENT_DIR"

# Clean up temp directory
rm -rf "$TEMP_DIR"