#!/bin/sh
set -eu

# Task — install script
# Installs both the task CLI and taskfile-lsp language server
# Usage: curl -fsSL https://raw.githubusercontent.com/youpkoopmansdev/taskfile/main/install/install.sh | sh

REPO="youpkoopmansdev/taskfile"
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

  install_binary "task"
  install_binary "taskfile-lsp"

  echo ""
  echo "✓ Task ${version} installed to ${INSTALL_DIR}/"
  echo "  Binaries: task, taskfile-lsp"
  echo "  Run 'task' in any directory with a Taskfile to get started."
}

install_binary() {
  bin="$1"
  if [ ! -f "${tmp}/${bin}" ]; then
    echo "  Skipping ${bin} (not found in archive)"
    return
  fi

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmp}/${bin}" "${INSTALL_DIR}/${bin}"
  else
    echo "Installing ${bin} to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${tmp}/${bin}" "${INSTALL_DIR}/${bin}"
  fi
  chmod +x "${INSTALL_DIR}/${bin}"
}

fetch_latest_version() {
  # Use the redirect from /releases/latest to avoid API rate limits
  if command -v curl >/dev/null 2>&1; then
    url=$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" 2>/dev/null)
  else
    url=$(wget --max-redirect=0 -qO /dev/null "https://github.com/${REPO}/releases/latest" 2>&1 | grep -oP 'Location: \K\S+' || true)
  fi
  version=$(echo "$url" | grep -oE '[^/]+$')
  if [ -z "$version" ]; then
    echo "Error: could not determine latest version. Check https://github.com/${REPO}/releases" >&2
    exit 1
  fi
  echo "$version"
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
