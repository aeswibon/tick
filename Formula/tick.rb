# Homebrew formula for: brew tap <owner>/tick && brew install tick
# Release workflow also publishes tick.rb on each GitHub release.
class Tick < Formula
  desc "Jira TUI dashboard for the terminal"
  homepage "https://github.com/aeswibon/tick"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/aeswibon/tick/releases/download/v0.1.0/tick-x86_64-apple-darwin"
      sha256 "REPLACE_ON_RELEASE"
    end
    on_arm do
      url "https://github.com/aeswibon/tick/releases/download/v0.1.0/tick-aarch64-apple-darwin"
      sha256 "REPLACE_ON_RELEASE"
    end
  end

  on_linux do
    url "https://github.com/aeswibon/tick/releases/download/v0.1.0/tick-x86_64-unknown-linux-gnu"
    sha256 "REPLACE_ON_RELEASE"
  end

  def install
    if OS.mac? && Hardware::CPU.arm?
      bin.install "tick-aarch64-apple-darwin" => "tick"
    elsif OS.mac?
      bin.install "tick-x86_64-apple-darwin" => "tick"
    else
      bin.install "tick-x86_64-unknown-linux-gnu" => "tick"
    end
  end

  test do
    assert_match "tick", shell_output("#{bin}/tick --help", 0)
  end
end
