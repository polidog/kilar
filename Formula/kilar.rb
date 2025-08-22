class Kilar < Formula
  desc "Powerful CLI tool for managing port processes"
  homepage "https://github.com/polidog/kilar"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_INTEL"
    else
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    else
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM"
    end
  end

  def install
    bin.install "kilar"
  end

  test do
    # Test basic command execution
    assert_match "kilar #{version}", shell_output("#{bin}/kilar --version")
    
    # Test help command
    assert_match "A powerful CLI tool for managing port processes", shell_output("#{bin}/kilar --help")
    
    # Test list command (should work without errors)
    system "#{bin}/kilar", "list", "--help"
  end
end