#!/bin/sh
# Installs the latest probeg release: curl -fsSL <this file's URL> | sh
set -eu

repo="guitvcer/probeg"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "probeg only runs on macOS." >&2
  exit 1
fi

case "$(uname -m)" in
  arm64) target="aarch64-apple-darwin" ;;
  x86_64) target="x86_64-apple-darwin" ;;
  *)
    echo "Unsupported processor: $(uname -m)" >&2
    exit 1
    ;;
esac

if [ -w /usr/local/bin ]; then
  dir="/usr/local/bin"
else
  dir="$HOME/.local/bin"
  mkdir -p "$dir"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

archive="probeg-$target.tar.gz"
base="https://github.com/$repo/releases/latest/download"
curl -fsSL "$base/$archive" -o "$work/$archive"
curl -fsSL "$base/$archive.sha256" -o "$work/$archive.sha256"
(cd "$work" && shasum -a 256 -c "$archive.sha256" >/dev/null)
tar -xzf "$work/$archive" -C "$work"
install -m 755 "$work/probeg" "$dir/probeg"

echo "Installed probeg to $dir/probeg"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add it to your PATH: echo 'export PATH=\"$dir:\$PATH\"' >> ~/.zshrc" ;;
esac
