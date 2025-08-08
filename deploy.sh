#!/bin/bash

DEFAULT="\033[0m"
RED="\033[0;31m"
GREEN="\033[0;32m"

get_binary_suffix() {
  case "$1" in
    "x86_64-unknown-linux-gnu")
      echo "-64bit-linux"
      ;;
    "aarch64-unknown-linux-gnu")
      echo "-arm64-linux"
      ;;
    "i686-unknown-linux-gnu")
      echo "-32bit-linux"
      ;;
    "armv7-unknown-linux-gnueabihf")
      echo "-arm-linux"
      ;;
    "aarch64-apple-darwin")
      echo "-arm64-macos"
      ;;
    "x86_64-apple-darwin")
      echo "-64bit-macos"
      ;;
    "x86_64-pc-windows-msvc")
      echo "-64bit-windows.exe"
      ;;
    *)
      echo ""
      ;;
  esac
}

targets=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
  "i686-unknown-linux-gnu"
  "armv7-unknown-linux-gnueabihf"
  "aarch64-apple-darwin"
  "x86_64-apple-darwin"
  "x86_64-pc-windows-msvc"
)


PACKAGE_NAMES=(
    "staticmcp_sse_dynamic"
    "staticmcp_sse_fixed"
)

echo "Building for all targets..."

CONCLUSION="\n\nBuild Summary:\n"

for target in "${targets[@]}"; do
    echo ""
    echo "Building for target: $target"

    if cargo build --release --target "$target"; then
        CONCLUSION+="$GREEN""✓""$DEFAULT Build successful for $target\n"

        for PACKAGE_NAME in "${PACKAGE_NAMES[@]}"; do
            binary_name="$PACKAGE_NAME"$(get_binary_suffix "$target")
            if [[ "$target" == *"windows"* ]]; then
                source_binary="target/$target/release/$PACKAGE_NAME.exe"
            else
                source_binary="target/$target/release/$PACKAGE_NAME"
            fi
            
            dest_binary="release/$binary_name"
            mkdir -p release
            if cp "$source_binary" "$dest_binary"; then
                CONCLUSION+="$GREEN""✓""$DEFAULT Copied to: release/$binary_name\n"
            else
                CONCLUSION+="$RED""✗""$DEFAULT Failed to copy binary for $target $PACKAGE_NAME\n"
            fi
        done        
    else
        CONCLUSION+="$RED""✗""$DEFAULT Build failed for $target\n"
    fi
done

echo -e "$CONCLUSION"