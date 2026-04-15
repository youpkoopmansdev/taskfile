#!/bin/sh
set -eu

# Task — install script
# Usage: curl -fsSL https://taskfile.koopmans.dev/install.sh | sh

REPO="youpkoopmansdev/taskfile"
BIN_NAME="task"
INSTALL_DIR="/usr/local/bin"

main() {
  platform=$(detect_platform)
  arch=$(detect_arch)
  target="${platform}-${arch}"

  echo "Detected: ${target}"

  version=$(fetch_latest_version)
  echo "Latest version: ${version}"

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  if [ "$platform" = "windows" ]; then
    filename="task-${target}.zip"
  else
    filename="task-${target}.tar.gz"
  fi

  url="https://github.com/${REPO}/releases/download/${version}/${filename}"
  echo "Downloading ${url}..."

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "${tmp}/${filename}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${tmp}/${filename}" "$url"
  else
    echo "Error: curl or wget is required" >&2
    exit 1
  fi

  if [ "$platform" = "windows" ]; then
    unzip -q "${tmp}/${filename}" -d "$tmp"
  else
    tar -xzf "${tmp}/${filename}" -C "$tmp"
  fi

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmp}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
  else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${tmp}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
  fi

  chmod +x "${INSTALL_DIR}/${BIN_NAME}"

  echo ""
  echo "✓ Task ${version} installed to ${INSTALL_DIR}/${BIN_NAME}"
  echo "  Run 'task' in any directory with a Taskfile to get started."
}

fetch_latest_version() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep '"tag_name"' | head -1 | cut -d'"' -f4
  else
    wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep '"tag_name"' | head -1 | cut -d'"' -f4
  fi
}

detect_platform() {
  case "$(uname -s)" in
    Linux*)   echo "linux" ;;
    Darwin*)  echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *)
      echo "Error: unsupported platform $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)   echo "x86_64" ;;
    aarch64|arm64)   echo "aarch64" ;;
    *)
      echo "Error: unsupported architecture $(uname -m)" >&2
      exit 1
      ;;
  esac
}

main
