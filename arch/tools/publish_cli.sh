#!/bin/bash

# Publishing script for plano CLI
# Supports publishing to:
# - PyPI (default)
# - GitHub Packages PyPI registry

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Parse command-line arguments
REGISTRY="pypi"
BUILD_ONLY=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --github)
      REGISTRY="github"
      shift
      ;;
    --pypi)
      REGISTRY="pypi"
      shift
      ;;
    --build-only)
      BUILD_ONLY=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --pypi         Publish to PyPI.org (default)"
      echo "  --github       Publish to GitHub Packages PyPI registry"
      echo "  --build-only   Only build the package, don't publish"
      echo "  -h, --help     Show this help message"
      echo ""
      echo "Examples:"
      echo "  $0                      # Publish to PyPI.org"
      echo "  $0 --github             # Publish to GitHub Packages"
      echo "  $0 --build-only         # Only build the package"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use -h or --help for usage information"
      exit 1
      ;;
  esac
done

echo "🔨 Building plano package..."
poetry build

if [ "$BUILD_ONLY" = true ]; then
  echo "✅ Build complete. Package files are in dist/"
  exit 0
fi

if [ "$REGISTRY" = "github" ]; then
  echo "📦 Publishing to GitHub Packages PyPI registry..."
  
  # Check for GitHub token
  if [ -z "$GITHUB_TOKEN" ]; then
    echo "❌ Error: GITHUB_TOKEN environment variable is not set"
    echo "Please set it with: export GITHUB_TOKEN=your_github_token"
    echo "Or create a token at: https://github.com/settings/tokens"
    exit 1
  fi
  
  # Configure poetry to use GitHub Packages
  poetry config repositories.github https://pypi.pkg.github.com/katanemo
  poetry config http-basic.github __token__ "$GITHUB_TOKEN"
  
  # Publish to GitHub Packages
  poetry publish --repository github
  
  echo "✅ Successfully published to GitHub Packages!"
  echo ""
  echo "Install with:"
  echo "  pip install plano --index-url https://pypi.pkg.github.com/katanemo/simple/"
  echo ""
  echo "Or with authentication:"
  echo "  pip install plano --index-url https://\${GITHUB_TOKEN}@pypi.pkg.github.com/katanemo/simple/"
  
elif [ "$REGISTRY" = "pypi" ]; then
  echo "📦 Publishing to PyPI.org..."
  
  # Publish to PyPI
  poetry publish
  
  echo "✅ Successfully published to PyPI.org!"
  echo ""
  echo "Install with:"
  echo "  pip install plano"
fi
