class Kilar < Formula
  desc "Powerful CLI tool for managing port processes"
  homepage "https://github.com/polidog/kilar"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "3479238c0490db363a3a9ca0aae68c7629be2625645713c42b57636d5de939a7"
    else
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "a29f7682871fb492adf440d3084de2ea52125ce7f0cbe36be7840d868cb28b14"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/polidog/kilar/releases/download/v#{version}/kilar-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "b6d976015ccfdf073b2546c214b966af588ab8641e9e27d2d7ad5ad9bd9c5e2a"
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