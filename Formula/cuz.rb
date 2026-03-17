class Cuz < Formula
  desc "Give every piece of code a traceable reason for existing"
  homepage "https://github.com/exla-ai/cuz"
  license "MIT"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-aarch64-apple-darwin.tar.gz"
      # sha256 will be filled by release workflow
    end
    on_intel do
      url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-x86_64-apple-darwin.tar.gz"
    end
  end

  on_linux do
    url "https://github.com/exla-ai/cuz/releases/download/v#{version}/cuz-x86_64-unknown-linux-gnu.tar.gz"
  end

  def install
    bin.install "cuz"
  end

  def post_install
    system bin/"cuz", "setup"
  end

  def caveats
    <<~EOS
      cuz has been installed and configured automatically:
        • ~/.claude/CLAUDE.md patched with intent tracking instructions
        • PostToolUse hook installed in ~/.claude/settings.json

      Run `cuz status` in any git repo to check tracking coverage.
    EOS
  end

  test do
    assert_match "cuz", shell_output("#{bin}/cuz --version")
  end
end
