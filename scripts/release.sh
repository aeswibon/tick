#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "  e.g. $0 v0.1.0"
  exit 1
fi

if ! git diff --quiet; then
  echo "Working tree has uncommitted changes. Commit or stash first."
  exit 1
fi

ARCHIVE_DIR="releases/$VERSION"
mkdir -p "$ARCHIVE_DIR"

build_target() {
  local target="$1"
  local archive_name="tick-${target}"
  echo "==> Building $target..."

  rustup target add "$target" 2>/dev/null || true
  if [ "$target" = "x86_64-unknown-linux-musl" ]; then
    CC_x86_64_unknown_linux_musl="x86_64-linux-musl-gcc" \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="x86_64-linux-musl-gcc" \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_AR="x86_64-linux-musl-ar" \
      cargo build --release --target "$target"
  else
    cargo build --release --target "$target"
  fi

  echo "==> Packaging $target..."
  cp "target/$target/release/tick" "$ARCHIVE_DIR/$archive_name"
  echo "$(sha256sum "$ARCHIVE_DIR/$archive_name" | cut -d' ' -f1)" > "$ARCHIVE_DIR/$archive_name.sha256"
}

build_target x86_64-apple-darwin
build_target aarch64-apple-darwin
build_target x86_64-unknown-linux-musl

echo "==> Generating checksums..."
cd "$ARCHIVE_DIR"
sha256sum tick-* > CHECKSUMS.txt 2>/dev/null || true
cd - > /dev/null

echo "==> Generating Homebrew formula..."
AMD64_SHA=$(cat "$ARCHIVE_DIR/tick-x86_64-apple-darwin.sha256")
ARM64_SHA=$(cat "$ARCHIVE_DIR/tick-aarch64-apple-darwin.sha256")
LINUX_SHA=$(cat "$ARCHIVE_DIR/tick-x86_64-unknown-linux-musl.sha256")

cat > "$ARCHIVE_DIR/tick.rb" <<EOF
class Tick < Formula
  desc "Jira TUI dashboard for the terminal"
  homepage "https://github.com/your-org/jira-cli"
  license "MIT"
  version "${VERSION#v}"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/your-org/jira-cli/releases/download/${VERSION}/tick-aarch64-apple-darwin"
      sha256 "${ARM64_SHA}"
    else
      url "https://github.com/your-org/jira-cli/releases/download/${VERSION}/tick-x86_64-apple-darwin"
      sha256 "${AMD64_SHA}"
    end
  end

  on_linux do
    url "https://github.com/your-org/jira-cli/releases/download/${VERSION}/tick-x86_64-unknown-linux-musl"
    sha256 "${LINUX_SHA}"
  end

  def install
    bin.install "tick"
  end

  test do
    system "\#{bin}/tick", "--help"
  end
end
EOF

echo ""
echo "==> Done! Release artifacts in $ARCHIVE_DIR/"
echo ""
ls -lh "$ARCHIVE_DIR/"
