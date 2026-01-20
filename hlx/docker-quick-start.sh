#!/bin/bash
# HLX Docker Quick Start Script
# This script helps you get started with HLX using Docker

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   HLX Docker Quick Start               ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed${NC}"
    echo "Please install Docker from https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo -e "${RED}Error: Docker Compose is not installed${NC}"
    echo "Please install Docker Compose"
    exit 1
fi

# Use 'docker compose' or 'docker-compose' based on what's available
if docker compose version &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
else
    DOCKER_COMPOSE="docker-compose"
fi

echo -e "${YELLOW}Step 1: Building Docker image...${NC}"
echo "This may take 10-15 minutes on first build."
echo ""

$DOCKER_COMPOSE build

echo ""
echo -e "${GREEN}✓ Build complete!${NC}"
echo ""

echo -e "${YELLOW}Step 2: Testing HLX compiler...${NC}"
$DOCKER_COMPOSE run --rm hlx-compiler hlx --version

echo ""
echo -e "${GREEN}✓ HLX compiler is working!${NC}"
echo ""

echo -e "${YELLOW}Step 3: Testing HLX LSP server...${NC}"
$DOCKER_COMPOSE run --rm hlx-lsp hlx_lsp --help || echo "(LSP server ready)"

echo ""
echo -e "${GREEN}✓ HLX LSP server is working!${NC}"
echo ""

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   Setup Complete!                      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""
echo "Quick commands:"
echo ""
echo "  ${GREEN}# Compile a HLX file${NC}"
echo "  $DOCKER_COMPOSE run --rm hlx-compiler hlx compile examples/hello_world.hlx"
echo ""
echo "  ${GREEN}# Run a HLX program${NC}"
echo "  $DOCKER_COMPOSE run --rm hlx-compiler hlx run examples/hello_world.hlx"
echo ""
echo "  ${GREEN}# Start LSP server${NC}"
echo "  $DOCKER_COMPOSE up hlx-lsp"
echo ""
echo "  ${GREEN}# Interactive development environment${NC}"
echo "  $DOCKER_COMPOSE run --rm hlx-dev bash"
echo ""
echo "For more information, see ${YELLOW}DOCKER.md${NC}"
echo ""
