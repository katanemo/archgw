# Claude Code Routing with Intelligence

## Why This Matters

**Claude Code is powerful, but what if you could access the best of ALL AI models through one familiar interface?**

Instead of being locked into a single provider, imagine:
- Using **DeepSeek's coding expertise** for complex algorithms
- Leveraging **GPT-4's reasoning** for architecture decisions
- Tapping **Claude's analysis** for code reviews
- Accessing **Grok's speed** for quick iterations

**All through the same Claude Code interface you already love.**

## The Problem with Single-Model Development

Most developers are stuck in single-provider silos:
- 🔒 **Vendor Lock-in**: Tied to one model's strengths and weaknesses
- 🎯 **Wrong Tool for the Job**: Using a reasoning model for simple tasks (expensive) or a fast model for complex problems (poor results)
- 🚫 **No Fallbacks**: When your preferred model is down, you're stuck
- 💸 **Suboptimal Costs**: Paying premium prices for tasks that could use cheaper models

## The Solution: Intelligent Multi-LLM Routing

Arch Gateway transforms Claude Code into a **universal AI development interface** that:

### 🌐 **Connects to Any LLM Provider**
- **OpenAI**: GPT-4o, o1-preview, GPT-4o-mini
- **Anthropic**: Claude 3.5 Sonnet, Claude 3 Haiku
- **DeepSeek**: DeepSeek-V3, DeepSeek-Coder-V2
- **Grok**: Grok-2, Grok-2-mini
- **Others**: Gemini, Llama, Mistral, local models via Ollama

### 🧠 **Routes Intelligently Based on Task**
Our research-backed routing system automatically selects the optimal model by analyzing:
- **Task complexity** (simple refactoring vs. architectural design)
- **Content type** (code generation vs. debugging vs. documentation)
- **Performance preferences** (speed vs. quality vs. cost)
- **Real-time availability** (automatic failover when models are down)

### 💡 **Learns Your Preferences**
The system adapts to your coding patterns and preferences over time, ensuring you always get the best model for your specific needs.

## Quick Start

### Prerequisites
- Claude Code installed: `npm install -g @anthropic-ai/claude-code`
- Docker running on your system

### 1. Install and Start Arch Gateway
```bash
pip install archgw
archgw up
```

### 2. Launch Claude Code with Multi-LLM Support
```bash
archgw cli-agent claude
```

That's it! Claude Code now has access to multiple LLM providers with intelligent routing.

## What You'll Experience

### Screenshot Placeholder
![Claude Code with Multi-LLM Routing](screenshot-placeholder.png)
*Claude Code interface enhanced with intelligent model routing and multi-provider access*

### Real-Time Model Selection
When you interact with Claude Code, you'll see:
- **Automatic model selection** based on your query type
- **Transparent routing decisions** showing which model was chosen and why
- **Seamless failover** if a model becomes unavailable
- **Performance metrics** comparing response times and quality

### Example Interactions

**Code Generation Query:**
```
You: "Create a Python function to validate email addresses"
→ Routed to: DeepSeek-Coder-V2 (optimized for code generation)
```

**Architecture Discussion:**
```
You: "How should I structure a microservices backend?"
→ Routed to: Claude 3.5 Sonnet (excellent for architectural reasoning)
```

**Quick Bug Fix:**
```
You: "Fix this syntax error in my JavaScript"
→ Routed to: GPT-4o-mini (fast and cost-effective for simple fixes)
```

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
ANTHROPIC_SMALL_FAST_MODEL=arch.fast.v1    # Uses intelligent alias
```

## Benefits You'll See Immediately

### 🚀 **Better Performance**
- Right model for each task = better results
- Automatic failover = no interruptions
- Caching = faster repeated queries

### 💰 **Cost Optimization**
- Use expensive models only when needed
- Leverage free/cheap models for simple tasks
- Track usage across all providers

### 🛡️ **Reliability**
- Multiple providers = no single point of failure
- Automatic retry logic
- Graceful degradation when models are unavailable

### 📊 **Insights**
- See which models work best for your coding style
- Track performance metrics across providers
- Optimize your model usage over time

## Real Developer Workflows

This intelligent routing is powered by our research in preference-aligned AI systems:
- **Research Paper**: [Preference-Aligned LLM Router](https://katanemo.com/research)
- **Technical Docs**: [docs.katanemo.com](https://docs.katanemo.com)
- **API Reference**: [docs.katanemo.com/api](https://docs.katanemo.com/api)
