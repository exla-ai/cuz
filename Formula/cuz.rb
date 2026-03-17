class Cuz < Formula
  desc "Give every piece of code a traceable reason for existing"
  homepage "https://github.com/exla-ai/cuz"
  license "MIT"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-aarch64-apple-darwin.tar.gz"
      sha256 "35c404b9a77d9911b6878c32c8c638c9b7d58a589bcaca9fd58f4657fc2eacff"
    end
    on_intel do
      url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-x86_64-apple-darwin.tar.gz"
      sha256 "e0493fcafad6a278b674d1f318f8ad6bf368ff26d7bde05110bde76b5d3858d4"
    end
  end

  on_linux do
    url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "797fbb606f7fd6bca45f7ec6eaeaf6eee1fe70050f1ed440f766bfc987e68060"
  end

  def install
    bin.install "cuz"
  end

  def post_install
    system bin/"cuz", "setup"
  end

  def caveats
    <<~EOS
      cuz is ready. Every Claude Code session will now automatically:
        • Track why code changes are made
        • Create intent records on each commit
        • Read existing intents before modifying code

      Run `cuz status` in any git repo to check tracking coverage.
    EOS
  end

  test do
    assert_match "cuz", shell_output("#{bin}/cuz --version")
  end
end
