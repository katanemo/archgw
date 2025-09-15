#!/bin/bash

# Model Alias Test Runner
# Usage: ./run_tests.sh [alias|direct|legacy|all]


set -e

# Load environment variables from .env if present
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Model Alias Test Runner${NC}"
echo "=================================="

# Check if archgw services are running
echo -e "${YELLOW}Checking if archgw services are running...${NC}"

if ! curl -s http://localhost:12000/health > /dev/null 2>&1; then
    echo -e "${RED}❌ LLM Gateway not running on port 12000${NC}"
    echo "Please start archgw services first:"
    echo "  export ARCH_CONFIG_PATH_RENDERED=$(pwd)/arch_config_with_aliases.yaml"
    echo "  # Start your archgw services"
    exit 1
fi

echo -e "${GREEN}✅ Services appear to be running${NC}"

# Check environment variables
echo -e "${YELLOW}Checking environment variables...${NC}"

if [ -z "$OPENAI_API_KEY" ]; then
    echo -e "${RED}❌ OPENAI_API_KEY not set${NC}"
    exit 1
fi

if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}⚠️  ANTHROPIC_API_KEY not set (some tests may fail)${NC}"
fi

echo -e "${GREEN}✅ Environment variables configured${NC}"

# Install dependencies if needed
if [ ! -d ".venv" ] && [ ! -f "poetry.lock" ]; then
    echo -e "${YELLOW}Installing dependencies...${NC}"
    poetry install
fi

# Set default test type
TEST_TYPE=${1:-alias}

echo -e "${BLUE}Running ${TEST_TYPE} tests...${NC}"
echo ""

# Run the tests
python model_alias.py "$TEST_TYPE"

echo ""
echo -e "${GREEN}Test run completed!${NC}"

# Show useful commands
echo ""
echo -e "${BLUE}Useful commands:${NC}"
echo "  ./run_tests.sh alias   - Run alias tests only"
echo "  ./run_tests.sh direct  - Run direct model tests"
echo "  ./run_tests.sh legacy  - Run legacy tests"
echo "  ./run_tests.sh all     - Run all tests"
echo ""
echo "  poetry run pytest -v -s model_alias.py::test_function_name  - Run specific test"
echo "  poetry run pytest -v -s -k 'alias'  - Run tests matching pattern"
