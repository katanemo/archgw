# Manual Testing & Development Guide for CLI

This guide explains how to manually test and develop the `archgw` CLI tool.

## Quick Start - Development Setup

### 1. Set Up Development Environment

From the `arch/tools` directory:

```bash
# Create and activate virtual environment
python -m venv venv
source venv/bin/activate  # On Mac/Linux
# OR
venv\Scripts\activate  # On Windows

# Install dependencies in development mode
poetry install
```

### 2. Run CLI in Development Mode

You have two options to run the CLI during development:

#### Option A: Run directly with Python (Fastest for iteration)

```bash
# From arch/tools directory
python -m cli.main --help
python -m cli.main build
python -m cli.main up --path /path/to/config
```

#### Option B: Install in editable mode (Recommended)

```bash
# Install the package in editable/development mode
pip install -e .

# Now you can use 'archgw' command directly
archgw --help
archgw build
archgw up --path /path/to/config
```

**Note**: After making changes to the code, you don't need to reinstall - changes are immediately available with editable install!

## Manual Testing Workflow

### Testing Individual Commands

Here's how to test each CLI command manually:

### 1. Test `build` Command

```bash
# Test building the Docker image
archgw build

# Or with Python directly
python -m cli.main build
```

**What to check:**
- Docker image builds successfully
- No errors in output
- Image is created with correct tag

### 2. Test `up` Command

```bash
# Test with a config file path
archgw up --path /path/to/demo/arch_config.yaml

# Test with explicit file
archgw up /path/to/arch_config.yaml

# Test with foreground mode
archgw up --path /path/to/config --foreground

# Test error case - non-existent file
archgw up --path /nonexistent/path
```

**What to check:**
- Config file is found correctly
- Validation runs
- Docker container starts
- Health checks pass
- Error messages are clear when things fail

### 3. Test `down` Command

```bash
# Stop the gateway
archgw down
```

**What to check:**
- Container stops cleanly
- No errors on shutdown

### 4. Test `logs` Command

```bash
# View logs
archgw logs

# Follow logs in real-time
archgw logs --follow

# View debug logs
archgw logs --debug --follow
```

**What to check:**
- Logs are readable
- Follow mode works correctly
- Debug mode shows additional information

### 5. Test `generate_prompt_targets` Command

```bash
# Generate targets from a Python file
archgw generate-prompt-targets --f /path/to/your/file.py
```

**What to check:**
- Targets are generated correctly
- Output format is valid
- Handles different Python function signatures

### 6. Test `cli_agent` Command

```bash
# Start CLI agent (requires archgw to be running)
archgw cli-agent claude --path /path/to/config

# With custom settings
archgw cli-agent claude --path /path/to/config --settings '{"key": "value"}'
```

**What to check:**
- Agent connects to running gateway
- Settings are passed correctly
- Error handling when gateway isn't running

### 7. Test Version Command

```bash
# Check version
archgw --version
```

## Development Tips

### Quick Iteration Cycle

1. **Make changes** to CLI code in `cli/` directory
2. **Test immediately** - no reinstall needed with editable install:
   ```bash
   python -m cli.main <command>
   # OR if installed
   archgw <command>
   ```
3. **Check output** and iterate

### Testing with Real Config Files

Use the demo configs in the repo:

```bash
# From arch/tools directory
cd ../../demos/samples_python/weather_forecast

# Test with this config
archgw up arch_config.yaml

# Or test from tools directory
archgw up ../../demos/samples_python/weather_forecast/arch_config.yaml
```

### Debugging Tips

1. **Add print statements** for quick debugging:
   ```python
   import sys
   print(f"DEBUG: Variable value: {variable}", file=sys.stderr)
   ```

2. **Use Python debugger**:
   ```python
   import pdb; pdb.set_trace()
   ```

3. **Check Docker status** manually:
   ```bash
   docker ps
   docker logs archgw
   docker inspect archgw
   ```

4. **Test individual functions** in Python REPL:
   ```bash
   python
   >>> from cli.utils import find_config_file
   >>> find_config_file(".", None)
   ```

### Common Test Scenarios

#### Test Error Handling

```bash
# Test missing config file
archgw up --path /nonexistent

# Test invalid config file
archgw up /path/to/invalid_config.yaml

# Test missing environment variables
# (remove required env vars and test)
archgw up --path /path/to/config
```

#### Test Edge Cases

```bash
# Test with minimal config
archgw up --path /path/to/minimal_config.yaml

# Test with complex config
archgw up --path /path/to/complex_config.yaml

# Test with different paths
archgw up --path .
archgw up --path /absolute/path
archgw up relative/path/config.yaml
```

## Example: Testing a Specific Feature

Let's say you're working on improving the `find_config_file` function. Here's how to test it:

```bash
# Start Python REPL
python

# Import and test
>>> from cli.utils import find_config_file
>>> import os
>>> find_config_file(".", None)
'/absolute/path/to/arch/tools/arch_config.yaml'

# Test with explicit file
>>> find_config_file(".", "/path/to/config.yaml")
'/path/to/config.yaml'

# Test error cases
>>> find_config_file("/nonexistent", None)
# Check what happens
```

## File Structure Reference

When developing, you'll primarily work with:

- `cli/main.py` - Main CLI commands and entry point
- `cli/core.py` - Core functionality (start_arch, stop_docker_container, etc.)
- `cli/utils.py` - Utility functions
- `cli/docker_cli.py` - Docker-related operations
- `cli/config_generator.py` - Config validation and generation
- `cli/targets.py` - Prompt target generation
- `cli/consts.py` - Constants

## Quick Reference: Common Commands

```bash
# Setup (one time)
cd arch/tools
python -m venv venv
source venv/bin/activate
poetry install
pip install -e .  # For editable install

# Development workflow
# 1. Edit code in cli/*.py
# 2. Test immediately:
python -m cli.main <command> [args]

# Or if installed:
archgw <command> [args]

# Common test commands:
archgw --version                    # Check version
archgw --help                       # See all commands
archgw build                        # Build Docker image
archgw up --path /path/to/config    # Start gateway
archgw down                         # Stop gateway
archgw logs --follow                # View logs
```

## Troubleshooting

### CLI command not found
- Make sure you activated the virtual environment
- If using editable install, verify: `pip list | grep archgw`
- Try: `python -m cli.main` instead

### Changes not reflected
- If using editable install, changes should be immediate
- If not, reinstall: `pip install -e .`
- Check you're editing the right file

### Docker issues
- Ensure Docker is running: `docker ps`
- Check container status: `docker ps -a | grep archgw`
- View logs: `docker logs archgw`

## Finding Demo Configs to Test With

The repository has several demo configs you can use for testing:

```bash
# List available demos
ls ../../demos/samples_python/*/arch_config.yaml

# Examples:
# - ../../demos/samples_python/weather_forecast/arch_config.yaml
# - ../../demos/samples_python/currency_exchange/arch_config.yaml
# - ../../demos/samples_python/human_resources_agent/arch_config.yaml
```

## Tips for Effective Manual Testing

1. **Test one thing at a time** - Make a small change, test it, then move on
2. **Test both success and failure cases** - Don't just test happy paths
3. **Use real configs** - Test with actual demo configs to catch real-world issues
4. **Check error messages** - Make sure error messages are helpful
5. **Test edge cases** - Empty files, missing fields, invalid values
6. **Keep notes** - Document what works and what doesn't as you develop

