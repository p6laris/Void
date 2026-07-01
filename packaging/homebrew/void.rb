class Void < Formula
  desc "Terminal focus timer with task tracking"
  homepage "https://github.com/p6laris/Void"
  version "@VERSION@"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/p6laris/Void/releases/download/@TAG@/void-macos-arm64.tar.gz"
      sha256 "@SHA_MAC_ARM64@"
    end
    on_intel do
      url "https://github.com/p6laris/Void/releases/download/@TAG@/void-macos-amd64.tar.gz"
      sha256 "@SHA_MAC_AMD64@"
    end
  end

  on_linux do
    url "https://github.com/p6laris/Void/releases/download/@TAG@/void-linux-amd64.tar.gz"
    sha256 "@SHA_LINUX@"
  end

  def install
    bin.install "void"
  end

  test do
    assert_match "Void", shell_output("#{bin}/void --help")
  end
end
