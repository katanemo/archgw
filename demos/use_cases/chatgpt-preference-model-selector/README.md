# 🗝️ RouteGPT (Beta)

**RouteGPT** is a dynamic model selector Chrome extension for ChatGPT. It intercepts your prompts, detects the user's intent, and automatically routes requests to the most appropriate model — based on preferences you define. Powered by the lightweight [Arch-Router](https://huggingface.co/katanemo/Arch-Router-1.5B.gguf), it makes multi-model usage seamless.

Think of it this way: changing models manually is like shifting gears on your bike every few pedals. RouteGPT automates that for you — so you can focus on the ride, not the mechanics.

---

## 📁 Project Name

Folder: `chatgpt-preference-model-selector`

---

## 🚀 Features

* 🧠 Preference-based routing (e.g., "code generation" → GPT-4, "travel help" → Gemini)
* 🤖 Local inference using [Ollama](https://ollama.com)
* 📙 Chrome extension interface for setting route preferences
* ⚡ Runs with [Arch-Router-1.5B.gguf](https://huggingface.co/katanemo/Arch-Router-1.5B.gguf)

---

## 📦 Installation

### 1. Clone and install dependencies

```
git clone https://github.com/your-org/chatgpt-preference-model-selector.git
cd chatgpt-preference-model-selector
npm install
```

### 2. Build the extension

```
npm install 
npm run build
```

This will create a `build/` directory that contains the unpacked Chrome extension.

---

## 🧠 Set Up Arch-Router in Ollama

Ensure [Ollama](https://ollama.com/download) is installed and running.

Then pull the Arch-Router model:

```
ollama pull hf.co/katanemo/Arch-Router-1.5B.gguf:Q4_K_M
```

Or run it directly:

```
ollama run hf.co/katanemo/Arch-Router-1.5B.gguf:Q4_K_M
```

### 🌐 Allow Chrome to Access Ollama

Start Ollama with appropriate network settings:

```
OLLAMA_ORIGINS=* OLLAMA_HOST=0.0.0.0 ollama serve
```

This:

* Binds Ollama to `0.0.0.0` for external access
* Sets CORS to allow requests from Chrome

---

## 📩 Load the Extension into Chrome

1. Open `chrome://extensions`
2. Enable **Developer mode** (top-right toggle)
3. Click **"Load unpacked"**
4. Select the `build/` folder inside `chatgpt-preference-model-selector`

Once loaded, RouteGPT will begin intercepting and routing your ChatGPT messages based on the preferences you define.

---

## ⚙️ Configure Routing Preferences

1. In ChatGPT, click the model dropdown.
2. A RouteGPT modal will appear.
3. Define your routing logic using natural language (e.g., `brainstorm startup ideas → gpt-4`, `summarize news articles → claude`).
4. Save your preferences. Routing begins immediately.

---

## 💸 Profit

RouteGPT helps you:

* Use expensive models only when needed
* Automatically shift to cheaper, faster, or more capable models based on task type
* Streamline multi-model workflows without extra clicks

---

## 🧪 Troubleshooting

* Make sure Ollama is reachable at `http://localhost:11434`
* If routing doesn’t seem to trigger, check DevTools console logs for `[ModelSelector]`
* Reload the extension and refresh the ChatGPT tab after updating preferences

---

## 🧱 Built With

* 🧠 [Arch-Router (1.5B)](https://huggingface.co/katanemo/Arch-Router-1.5B.gguf)
* 📙 Chrome Extensions API
* 🛠️ Ollama
* ⚛️ React + TypeScript

---

## 📜 License

Apache 2.0 © Katanemo Labs, Inc.
