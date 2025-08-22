# Homebrew Tap Setup Guide

This guide explains how to set up and maintain the Homebrew tap for kilar.

## Prerequisites

- GitHub CLI (`brew install gh`)
- GitHub account with repository creation permissions
- Homebrew installed on your system

## Initial Setup

### 1. Automatic Setup

Run the setup script to create the tap repository:

```bash
make tap-setup
```

Or directly:

```bash
./scripts/setup-homebrew-tap.sh
```

This will:
1. Create a new repository `homebrew-tap` on GitHub
2. Set up the proper directory structure
3. Copy the formula file
4. Push everything to GitHub

### 2. Manual Setup

If you prefer to set up manually:

1. Create a new GitHub repository named `homebrew-tap`
2. Clone it locally
3. Create the Formula directory structure:

```bash
git clone https://github.com/polidog/homebrew-tap.git
cd homebrew-tap
mkdir -p Formula
cp ../kilar/Formula/kilar.rb Formula/
cp ../kilar/homebrew-tap-README.md README.md
git add .
git commit -m "Initial tap setup"
git push origin main
```

## Updating the Formula

### After a New Release

When you create a new release on GitHub:

1. The GitHub Action will automatically create a PR to update the formula
2. Review and merge the PR

### Manual Update

To manually update the formula with new SHA256 checksums:

```bash
# Update to version 0.1.1
make tap-update VERSION=0.1.1
```

Or directly:

```bash
./scripts/update-formula.sh v0.1.1
```

## Testing the Tap

### Local Testing

Before pushing changes, test the formula locally:

```bash
# Add your local tap
brew tap polidog/tap path/to/homebrew-tap

# Test installation
brew install --verbose --debug kilar

# Run tests
brew test kilar

# Audit the formula
brew audit --strict kilar
```

### Clean Installation Test

```bash
# Remove existing installation
brew uninstall kilar 2>/dev/null || true
brew untap polidog/tap 2>/dev/null || true

# Fresh install
brew tap polidog/tap
brew install kilar
kilar --version
```

## Formula Structure

The formula (`Formula/kilar.rb`) contains:

- **Metadata**: Description, homepage, version, license
- **Download URLs**: Platform-specific binary URLs
- **SHA256 Checksums**: For integrity verification
- **Installation Instructions**: How to install the binary
- **Tests**: Basic functionality tests

## Troubleshooting

### SHA256 Mismatch

If users report SHA256 mismatch errors:

1. Download the release artifacts manually
2. Calculate correct SHA256: `sha256sum filename.tar.gz`
3. Update the formula with correct checksums
4. Push the fix

### Formula Syntax Errors

Validate the formula:

```bash
# Check Ruby syntax
ruby -c Formula/kilar.rb

# Brew audit
brew audit --strict Formula/kilar.rb
```

### Installation Failures

Common issues and solutions:

1. **Permission denied**: Ensure proper file permissions in the tar.gz
2. **Binary not found**: Check the tar.gz structure matches formula expectations
3. **Wrong architecture**: Verify platform detection logic in formula

## Maintenance Checklist

### For Each Release

- [ ] Create GitHub release with proper version tag (v0.1.0)
- [ ] Wait for release artifacts to be uploaded
- [ ] Verify GitHub Action creates update PR
- [ ] Test installation with new version
- [ ] Merge PR after successful tests
- [ ] Announce update to users

### Periodic Tasks

- [ ] Test formula on different platforms
- [ ] Update formula for Homebrew best practices
- [ ] Monitor user issues and feedback
- [ ] Keep documentation up to date

## User Installation

Users can install kilar via Homebrew:

```bash
# Standard installation
brew tap polidog/tap
brew install kilar

# One-line installation
brew install polidog/tap/kilar

# Update to latest version
brew update
brew upgrade kilar
```

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Taps Documentation](https://docs.brew.sh/Taps)
- [GitHub Actions for Homebrew](https://github.com/Homebrew/actions)

## Support

For issues with the Homebrew formula:
1. Check [existing issues](https://github.com/polidog/kilar/issues)
2. Create a new issue with the `homebrew` label
3. Include error messages and system information