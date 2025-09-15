# Model Alias Testing Suite

This directory contains comprehensive tests for the model alias feature in archgw.

## Overview

Model aliases allow clients to use friendly, semantic names instead of provider-specific model names. For example:
- `arch.summarize.v1` → `4o-mini` (fast, cheap model for summaries)
- `arch.reasoning.v1` → `gpt-4o` (capable model for complex reasoning)
- `creative-model` → `claude-3-5-sonnet` (creative tasks)

## Files

- `model_alias.py` - Main test suite with comprehensive alias tests
- `arch_config_with_aliases.yaml` - Configuration file with model aliases defined
- `pyproject.toml` - Python dependencies
- `README.md` - This file

## Test Categories

### 1. Model Alias Tests
Tests that verify alias resolution works correctly:
- OpenAI client with various aliases
- Anthropic client with various aliases
- Streaming and non-streaming requests
- Error handling for non-existent aliases

### 2. Direct Model Tests
Tests using direct model names (for comparison):
- Direct provider model names
- Verification that direct models still work

### 3. Legacy Tests
Existing tests to ensure backwards compatibility

## Configuration

The `arch_config_with_aliases.yaml` file defines several aliases:

```yaml
model_aliases:
  # Task-specific aliases
  arch.summarize.v1:
    target: 4o-mini
  arch.reasoning.v1:
    target: gpt-4o
  arch.creative.v1:
    target: claude-3-5-sonnet

  # Semantic aliases
  summary-model:
    target: 4o-mini
  chat-model:
    target: gpt-4o
```

## Running Tests

### Prerequisites

1. **Start archgw services** with the alias configuration:
   ```bash
   # From arch root directory
   export ARCH_CONFIG_PATH_RENDERED=/path/to/arch_config_with_aliases.yaml
   # Start your archgw services (brightstaff, llm_gateway, etc.)
   ```

2. **Set environment variables**:
   ```bash
   export OPENAI_API_KEY=your_openai_key
   export ANTHROPIC_API_KEY=your_anthropic_key
   export LLM_GATEWAY_ENDPOINT=http://localhost:12000/v1/chat/completions
   ```

3. **Install dependencies**:
   ```bash
   poetry install
   ```

### Run Tests

#### Run specific test categories:
```bash
# Run only model alias tests (default)
python model_alias.py alias

# Run only direct model tests
python model_alias.py direct

# Run only legacy tests
python model_alias.py legacy

# Run all tests
python model_alias.py all
```

#### Run individual tests with pytest:
```bash
# Run specific test function
poetry run pytest -v -s model_alias.py::test_openai_client_with_alias_arch_summarize_v1

# Run all alias tests
poetry run pytest -v -s -k "alias"

# Run with detailed logging
poetry run pytest -v -s --log-cli-level=INFO model_alias.py
```

## Expected Behavior

### When Alias Exists
1. Client sends request with alias (e.g., `"model": "arch.summarize.v1"`)
2. Brightstaff resolves alias to actual model (`arch.summarize.v1` → `4o-mini`)
3. Request is forwarded to LLM Gateway with resolved model name
4. Response is returned to client

### When Alias Doesn't Exist
1. Client sends request with unknown alias (e.g., `"model": "unknown.alias"`)
2. Brightstaff treats it as a direct model name
3. Request may succeed (if model exists) or fail (if model doesn't exist)

## Troubleshooting

### Test Failures

1. **Connection errors**: Ensure archgw services are running on expected ports
2. **Authentication errors**: Check API keys are set correctly
3. **Model not found**: Verify the target models in aliases exist in your config
4. **Alias not resolved**: Check alias is defined correctly in arch_config.yaml

### Debugging

1. **Check brightstaff logs** for alias resolution messages:
   ```
   Model alias resolved: 'arch.summarize.v1' -> '4o-mini'
   ```

2. **Verify configuration** is loaded:
   ```bash
   # Check if config file is being read
   grep -i "model_aliases" /path/to/arch_config_with_aliases.yaml
   ```

3. **Test with direct model names** first to ensure basic functionality works

## Adding New Tests

To add a new alias test:

1. Define the alias in `arch_config_with_aliases.yaml`
2. Add a test function in `model_alias.py`:
   ```python
   def test_my_new_alias():
       logger.info("Testing my new alias")
       # ... test implementation
   ```
3. Add the test to the appropriate test runner function

## Log Output

The test suite provides detailed logging:
- Test start/completion status
- API request/response details
- Alias resolution confirmations
- Error details for failed tests
- Summary statistics

Example output:
```
============================================================
RUNNING: OpenAI client with arch.summarize.v1 alias
============================================================
2024-09-14 10:30:15 - Testing OpenAI client with alias 'arch.summarize.v1' -> '4o-mini'
2024-09-14 10:30:16 - Response from arch.summarize.v1 alias: Hello from alias arch.summarize.v1!
✅ PASSED: OpenAI client with arch.summarize.v1 alias
```
