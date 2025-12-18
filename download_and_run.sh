#!/usr/bin/env bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO="relativemodder/geode-cli-installer-rs"
BINARY_NAME="geode-cli-installer"
INSTALL_DIR="/tmp"

echo -e "${GREEN}Geode CLI Installer${NC}"
echo "Downloading and running installer..."
echo


OS=$(uname -s)
ARCH=$(uname -m)

if [ "$OS" != "Linux" ] || [ "$ARCH" != "x86_64" ]; then
    echo -e "${RED}Error: This installer only supports Linux x86_64${NC}"
    exit 1
fi


DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}"

echo "Downloading installer..."
TEMP_FILE="${INSTALL_DIR}/${BINARY_NAME}"

if command -v curl &> /dev/null; then
    curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_FILE"
elif command -v wget &> /dev/null; then
    wget -q "$DOWNLOAD_URL" -O "$TEMP_FILE"
else
    echo -e "${RED}Error: Neither curl nor wget found. Please install one of them.${NC}"
    exit 1
fi

chmod +x "$TEMP_FILE"

echo -e "${GREEN}Running installer...${NC}"
echo
"$TEMP_FILE"

# cleanup
rm -f "$TEMP_FILE"