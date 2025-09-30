# Claude Code Routing with (Preference-aligned) Intelligence

## Why This Matters

**Claude Code is powerful, but what if you could access the best of ALL AI models through one familiar interface?**

Instead of being locked into a set of LLMs from one provier, imagine:
- Using **DeepSeek's coding expertise** for complex algorithms
- Leveraging **GPT-5's reasoning** for architecture decisions
- Tapping **Claude's analysis** for code reviews
- Accessing **Grok's speed** for quick iterations

**All through the same Claude Code interface you already love.**

## The Solution: Intelligent Multi-LLM Routing

Arch Gateway transforms Claude Code into a **universal AI development interface** that:

### 🌐 **Connects to Any LLM Provider**
- **OpenAI**: GPT-4.1, GPT-5, etc.
- **Anthropic**: Claude 3.5 Sonnet, Claude 3 Haiku, Claude 4.5
- **DeepSeek**: DeepSeek-V3, DeepSeek-Coder-V2
- **Grok**: Grok-2, Grok-2-mini
- **Others**: Gemini, Llama, Mistral, local models via Ollama

### 🧠 **Routes Intelligently Based on Task**
Our research-backed routing system automatically selects the optimal model by analyzing:
- **Task complexity** (simple refactoring vs. architectural design)
- **Content type** (code generation vs. debugging vs. documentation)


## Quick Start

### Prerequisites
- Claude Code installed: `npm install -g @anthropic-ai/claude-code`
- Docker running on your system
- Create a python virtual environment in your current working directory

### 1. Get the Configuration File
Download the demo configuration file using one of these methods:

**Option A: Direct download**
```bash
curl -O https://raw.githubusercontent.com/katanemo/arch/main/demos/use_cases/claude_code/config.yaml
```

**Option B: Clone the repository**
```bash
git clone https://github.com/katanemo/arch.git
cd arch/demos/use_cases/claude_code

```

### 2. Set Up Your API Keys
Set up your environment variables with your actual API keys:
```bash
export OPENAI_API_KEY="your-openai-api-key"
export ANTHROPIC_API_KEY="your-anthropic-api-key"
export AZURE_API_KEY="your-azure-api-key"  # Optional
```

Alternatively, create a `.env` file in your working directory:
```bash
echo "OPENAI_API_KEY=your-openai-api-key" > .env
echo "ANTHROPIC_API_KEY=your-anthropic-api-key" >> .env
```

### 3. Install and Start Arch Gateway
```bash
pip install archgw
archgw up
```

### 4. Launch Claude Code with Multi-LLM Support
```bash
archgw cli-agent claude
```

That's it! Claude Code now has access to multiple LLM providers with intelligent routing.

## What You'll Experience

### Screenshot Placeholder
![Claude Code with Multi-LLM Routing](screenshot-placeholder.png)
*Claude Code interface enhanced with intelligent model routing and multi-provider access*

### Real-Time Model Selection
When you interact with Claude Code, you'll get:
- **Automatic model selection** based on your query type
- **Transparent routing decisions** showing which model was chosen and why
- **Seamless failover** if a model becomes unavailable

## Configuration

The setup uses the included `config.yaml` file which defines:

### Multi-Provider Access
```yaml
llm_providers:
  - model: openai/gpt-4.1-2025-04-14
    access_key: $OPENAI_API_KEY
    routing_preferences:
    - name: code generation
        description: generating new code snippets and functions
  - model: anthropic/claude-3-5-sonnet-20241022
    access_key: $ANTHROPIC_API_KEY
    routing_preferences:
        name: code understanding
        description: explaining and analyzing existing code
```
## Advanced Usage

### Custom Model Selection
```bash
# Force a specific model for this session
archgw cli-agent claude --settings='{"ANTHROPIC_SMALL_FAST_MODEL": "deepseek-coder-v2"}'

# Enable detailed routing information
archgw cli-agent claude --settings='{"statusLine": {"type": "command", "command": "ccr statusline"}}'
```

### Environment Variables
The system automatically configures:
```bash
ANTHROPIC_BASE_URL=http://127.0.0.1:12000  # Routes through Arch Gateway
ANTHROPIC_SMALL_FAST_MODEL=arch.claude.code.small.fast    # Uses intelligent alias
```

## Real Developer Workflows

This intelligent routing is powered by our research in preference-aligned LLMM routing:
- **Research Paper**: [Preference-Aligned LLM Router](https://arxiv.org/abs/2506.16655)
- **Technical Docs**: [docs.archgw.com](https://docs.archgw.com)
