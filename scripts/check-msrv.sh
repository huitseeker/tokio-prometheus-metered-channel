#!/bin/bash
set -e
set -o pipefail

# MSRV checking script for single-crate repository

# ---- utilities --------------------------------------------------------------

check_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR: Required command '$1' is not installed or not in PATH"
    exit 1
  fi
}

# Check required commands
check_command "cargo"
check_command "rustup"

# ---- repo root -------------------------------------------------------------

# Get the directory where this script is located and change to the parent directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR/.."

echo "Checking MSRV..."

# Get the package name from Cargo.toml
package_name="$(grep '^name = ' Cargo.toml | head -n1 | sed 's/name = "\(.*\)"/\1/')"
echo "Package: $package_name"

# Get current MSRV from Cargo.toml
current_msrv="$(grep '^rust-version = ' Cargo.toml | sed 's/rust-version = "\(.*\)"/\1/')"

if [[ -z "$current_msrv" ]]; then
    echo "WARNING: No rust-version found in Cargo.toml"
    exit 1
fi

echo "Current MSRV: $current_msrv"

# Try to verify the MSRV
if ! cargo msrv verify >/dev/null 2>&1; then
    echo "ERROR: MSRV check failed for $package_name"

    echo "Searching for correct MSRV for $package_name..."

    # Determine the currently-installed stable toolchain version
    latest_stable="$(rustup run stable rustc --version 2>/dev/null | awk '{print $2}')"
    if [[ -z "$latest_stable" ]]; then latest_stable="1.81.0"; fi

    # Search for the actual MSRV starting from the current one
    if actual_msrv=$(cargo msrv find \
          --min "$current_msrv" \
          --max "$latest_stable" \
          --output-format minimal 2>/dev/null); then
      echo "Found actual MSRV: $actual_msrv"
      echo ""
      echo "ERROR SUMMARY for $package_name:"
      echo "Package:   $package_name"
      echo "Current (incorrect) MSRV: $current_msrv"
      echo "Correct MSRV:             $actual_msrv"
      echo ""
      echo "TO FIX:"
      echo "Update rust-version in Cargo.toml from \"$current_msrv\" to \"$actual_msrv\""
    else
      echo "Could not determine correct MSRV automatically"
      echo ""
      echo "ERROR SUMMARY for $package_name:"
      echo "Package:   $package_name"
      echo "Current (incorrect) MSRV: $current_msrv"
      echo "Could not automatically determine correct MSRV"
      echo ""
      echo "TO FIX:"
      echo "Run manually: cargo msrv find"
    fi
    echo "-------------------------------------------------------------------------------"
    exit 1
else
    echo "OK: MSRV check passed for $package_name"
    exit 0
fi
