#!/bin/sh
set -e

REPO="exla-ai/cuz"
VERSION="v0.1.0"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}" in
  Darwin)
    case "${ARCH}" in
      arm64)  TARGET="aarch64-apple-darwin" ;;
      x86_64) TARGET="x86_64-apple-darwin" ;;
      *)      echo "Unsupported architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "${ARCH}" in
      x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
      *)      echo "Unsupported architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: ${OS}" >&2; exit 1
    ;;
esac

URL="https://github.com/${REPO}/releases/download/${VERSION}/cuz-${TARGET}.tar.gz"

# Pick install dir: /usr/local/bin if writable, else ~/.local/bin
if [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -w "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
  INSTALL_DIR="$HOME/.local/bin"
else
  INSTALL_DIR="/usr/local/bin"
fi

echo "Downloading cuz ${VERSION} for ${TARGET}..."
TMP=$(mktemp -d)
curl -sL "${URL}" | tar xz -C "${TMP}"

echo "Installing to ${INSTALL_DIR}..."
if [ -w "${INSTALL_DIR}" ]; then
  mv "${TMP}/cuz" "${INSTALL_DIR}/cuz"
else
  sudo mv "${TMP}/cuz" "${INSTALL_DIR}/cuz"
fi
rm -rf "${TMP}"

# Ensure install dir is on PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "Note: Add ${INSTALL_DIR} to your PATH:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    export PATH="${INSTALL_DIR}:${PATH}"
    ;;
esac

echo "Running cuz setup..."
cuz setup

echo ""
echo "Done! Run 'cuz status' in any git repo to check tracking."
