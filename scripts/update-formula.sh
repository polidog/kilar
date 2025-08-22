#!/bin/bash
set -e

VERSION=$1
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

# Remove 'v' prefix if present
VERSION=${VERSION#v}

FORMULA_PATH="Formula/kilar.rb"
TEMP_DIR=$(mktemp -d)
TAP_DIR="$TEMP_DIR/homebrew-kilar"

echo "Updating Homebrew formula for version $VERSION..."

# Clone homebrew-kilar repository
echo "Cloning homebrew-kilar repository..."
cd "$TEMP_DIR"
if ! gh repo clone polidog/homebrew-kilar; then
    echo "Failed to clone homebrew-kilar repository"
    exit 1
fi

cd homebrew-kilar

# Function to download and calculate SHA256
calculate_sha256() {
    local url=$1
    local filename=$(basename "$url")
    
    echo "Downloading $url..."
    curl -L -o "$TEMP_DIR/$filename" "$url"
    
    if [ ! -f "$TEMP_DIR/$filename" ]; then
        echo "Failed to download $url"
        return 1
    fi
    
    sha256sum "$TEMP_DIR/$filename" | cut -d' ' -f1
}

# URLs for different platforms
BASE_URL="https://github.com/polidog/kilar/releases/download/v${VERSION}"
MACOS_INTEL_URL="${BASE_URL}/kilar-${VERSION}-x86_64-apple-darwin.tar.gz"
MACOS_ARM_URL="${BASE_URL}/kilar-${VERSION}-aarch64-apple-darwin.tar.gz"
LINUX_X86_URL="${BASE_URL}/kilar-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
LINUX_ARM_URL="${BASE_URL}/kilar-${VERSION}-aarch64-unknown-linux-gnu.tar.gz"

# Calculate SHA256 for each platform
echo "Calculating SHA256 checksums..."
MACOS_INTEL_SHA256=$(calculate_sha256 "$MACOS_INTEL_URL") || true
MACOS_ARM_SHA256=$(calculate_sha256 "$MACOS_ARM_URL") || true
LINUX_X86_SHA256=$(calculate_sha256 "$LINUX_X86_URL") || true
LINUX_ARM_SHA256=$(calculate_sha256 "$LINUX_ARM_URL") || true

# Update the formula file
echo "Updating formula file..."

# Update version
sed -i.bak "s/version \".*\"/version \"${VERSION}\"/" "$FORMULA_PATH"

# Update SHA256 values
if [ -n "$MACOS_INTEL_SHA256" ]; then
    sed -i.bak "s|sha256 \"PLACEHOLDER_SHA256_MACOS_INTEL\"|sha256 \"${MACOS_INTEL_SHA256}\"|" "$FORMULA_PATH"
    sed -i.bak "s|sha256 \"[a-f0-9]*\" # x86_64-apple-darwin|sha256 \"${MACOS_INTEL_SHA256}\" # x86_64-apple-darwin|" "$FORMULA_PATH"
fi

if [ -n "$MACOS_ARM_SHA256" ]; then
    sed -i.bak "s|sha256 \"PLACEHOLDER_SHA256_MACOS_ARM\"|sha256 \"${MACOS_ARM_SHA256}\"|" "$FORMULA_PATH"
    sed -i.bak "s|sha256 \"[a-f0-9]*\" # aarch64-apple-darwin|sha256 \"${MACOS_ARM_SHA256}\" # aarch64-apple-darwin|" "$FORMULA_PATH"
fi

if [ -n "$LINUX_X86_SHA256" ]; then
    sed -i.bak "s|sha256 \"PLACEHOLDER_SHA256_LINUX_X86_64\"|sha256 \"${LINUX_X86_SHA256}\"|" "$FORMULA_PATH"
    sed -i.bak "s|sha256 \"[a-f0-9]*\" # x86_64-unknown-linux-gnu|sha256 \"${LINUX_X86_SHA256}\" # x86_64-unknown-linux-gnu|" "$FORMULA_PATH"
fi

if [ -n "$LINUX_ARM_SHA256" ]; then
    sed -i.bak "s|sha256 \"PLACEHOLDER_SHA256_LINUX_ARM\"|sha256 \"${LINUX_ARM_SHA256}\"|" "$FORMULA_PATH"
    sed -i.bak "s|sha256 \"[a-f0-9]*\" # aarch64-unknown-linux-gnu|sha256 \"${LINUX_ARM_SHA256}\" # aarch64-unknown-linux-gnu|" "$FORMULA_PATH"
fi

# Remove backup files
rm -f "${FORMULA_PATH}.bak"

# Commit and push changes
echo "Committing and pushing changes..."
git add Formula/kilar.rb
git commit -m "Update kilar formula to version ${VERSION}

- Updated version to ${VERSION}
- Updated SHA256 checksums for all platforms"

git push origin main

echo "Formula updated successfully!"
echo ""
echo "SHA256 checksums:"
echo "  macOS Intel:  ${MACOS_INTEL_SHA256:-Not available}"
echo "  macOS ARM:    ${MACOS_ARM_SHA256:-Not available}"
echo "  Linux x86_64: ${LINUX_X86_SHA256:-Not available}"
echo "  Linux ARM:    ${LINUX_ARM_SHA256:-Not available}"

# Clean up temp directory
cd /
rm -rf "$TEMP_DIR"

echo ""
echo "✅ Homebrew formula updated in repository!"
echo "Repository: https://github.com/polidog/homebrew-kilar"