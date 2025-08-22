class Kilar < Formula
  desc "Powerful CLI tool for managing port processes"
  homepage "https://github.com/polidog/kilar"
  version "0.1.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "e59be0721183dc2c6652ff93eda50421ff08687a45202e9ec2a522ca272bdb60"
    else
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "3dc2839389c363915d037530e822e7d4c0fe9e74fa62565f63d591c0b21b58e4"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "ee07fb2101b32f5fcc712fee5c0a42a5060f4363267701459dabb032057d8abe"
    else
      # Linux ARM version is not currently available
      odie "Linux ARM64 version is not available. Please use x86_64 version or build from source."
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