#!/bin/sh
# install.sh — fetch the trikala CLI binary for macOS / Linux.
#
# Usage:
#   curl -fsSL https://trikala.round.online/install.sh | sh
#   curl -fsSL https://trikala.round.online/install.sh | sh -s -- v0.1.0-alpha.2
#
# Honors TRIKALA_INSTALL_DIR (default: ~/.local/bin).

set -eu

REPO="RoundOnline/trikala"
VERSION="${1:-latest}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    linux)  PLATFORM_OS="linux" ;;
    darwin) PLATFORM_OS="macos" ;;
    *)
        echo "[ATI-100] unsupported OS: $OS" >&2
        echo "  hint: install.sh covers linux + macOS. For Windows use install.ps1." >&2
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)   PLATFORM_ARCH="x86_64" ;;
    arm64|aarch64)  PLATFORM_ARCH="aarch64" ;;
    *)
        echo "[ATI-101] unsupported architecture: $ARCH" >&2
        echo "  hint: pre-built binaries exist for x86_64 and aarch64. Build from source if you're on another arch." >&2
        exit 1
        ;;
esac

# macOS x86_64 is supported, linux aarch64 is not (yet).
if [ "$PLATFORM_OS" = "linux" ] && [ "$PLATFORM_ARCH" = "aarch64" ]; then
    echo "[ATI-102] linux-aarch64 binaries not built yet" >&2
    echo "  hint: open an issue at https://github.com/${REPO}/issues if you need this target." >&2
    exit 1
fi

ASSET="trikala-${PLATFORM_OS}-${PLATFORM_ARCH}.tar.gz"

if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "→ fetching ${ASSET}"
if ! curl -fsSL "$URL" -o "${TMP}/${ASSET}"; then
    echo "[ATI-103] download failed: $URL" >&2
    echo "  hint: check version exists at https://github.com/${REPO}/releases" >&2
    exit 1
fi

tar -xzf "${TMP}/${ASSET}" -C "$TMP"

INSTALL_DIR="${TRIKALA_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"
mv "${TMP}/trikala" "${INSTALL_DIR}/trikala"
chmod +x "${INSTALL_DIR}/trikala"

echo
echo "✓ installed to ${INSTALL_DIR}/trikala"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo
        echo "${INSTALL_DIR} is NOT on your PATH. Add this line to your shell rc:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

echo
echo "Try: trikala --version"
