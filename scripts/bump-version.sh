#!/usr/bin/env bash
set -euo pipefail

# bump-version.sh — Update version across all project files
# Usage: ./scripts/bump-version.sh <version>
# Example: ./scripts/bump-version.sh 0.1.2

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>" >&2
  echo "Example: $0 0.1.2" >&2
  exit 1
fi

# Validate semver format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
  echo "Error: Invalid version format: $VERSION" >&2
  echo "Expected: X.Y.Z or X.Y.Z-prerelease" >&2
  exit 1
fi

# Check working tree is clean
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
  echo "Error: Working tree is not clean. Commit or stash changes first." >&2
  exit 1
fi

# Check version is higher than latest tag
LATEST_TAG=$(git tag --sort=-v:refname 2>/dev/null | head -1 || true)
if [ -n "$LATEST_TAG" ]; then
  LATEST="${LATEST_TAG#v}"
  HIGHER=$(printf '%s\n%s\n' "$LATEST" "$VERSION" | sort -V | tail -1)
  if [ "$HIGHER" != "$VERSION" ]; then
    echo "Error: Version $VERSION is not higher than latest tag $LATEST" >&2
    exit 1
  fi
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PKG_JSON="$PROJECT_ROOT/package.json"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"

for f in "$PKG_JSON" "$TAURI_CONF" "$CARGO_TOML"; do
  if [ ! -f "$f" ]; then
    echo "Error: File not found: $f" >&2
    exit 1
  fi
done

echo "Bumping version to $VERSION ..."

# package.json — line 4: "version": "X.Y.Z"
sed -i.bak '4s/"version": "[^"]*"/"version": "'"$VERSION"'"/' "$PKG_JSON"

# tauri.conf.json — line 4: "version": "X.Y.Z"
sed -i.bak '4s/"version": "[^"]*"/"version": "'"$VERSION"'"/' "$TAURI_CONF"

# Cargo.toml — line 3: version = "X.Y.Z"
sed -i.bak '3s/^version = "[^"]*"/version = "'"$VERSION"'"/' "$CARGO_TOML"

# Cleanup backup files
rm -f "$PKG_JSON.bak" "$TAURI_CONF.bak" "$CARGO_TOML.bak"

# Verify all three files match
PKG_VER=$(grep '"version"' "$PKG_JSON" | head -1 | grep -o '"[^"]*"$' | tr -d '"')
TAURI_VER=$(grep '"version"' "$TAURI_CONF" | head -1 | grep -o '"[^"]*"$' | tr -d '"')
CARGO_VER=$(grep '^version = ' "$CARGO_TOML" | head -1 | grep -o '"[^"]*"' | tr -d '"')

echo "  package.json:      $PKG_VER"
echo "  tauri.conf.json:   $TAURI_VER"
echo "  Cargo.toml:        $CARGO_VER"

if [ "$PKG_VER" != "$VERSION" ] || [ "$TAURI_VER" != "$VERSION" ] || [ "$CARGO_VER" != "$VERSION" ]; then
  echo "Error: Version mismatch after update!" >&2
  exit 1
fi

echo "All versions updated to $VERSION successfully."
