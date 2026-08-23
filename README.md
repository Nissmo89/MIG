# MIG (Make It Green)

MIG is an auto-contributing bot that runs locally (e.g., on system startup) and pushes code to your GitHub repository to help keep your contribution graph green!

It uses LangGraph/LangChain to maintain memory of its past contributions, ensuring it doesn't repeat itself and continues to grow.

## Features
- **Personality**: Driven by a `Context.md` file (default is a DSA enthusiast).
- **Memory**: Uses `SqliteSaver` in LangGraph to remember previous problems it solved.
- **LLM Support**: Supports Groq and OpenRouter (for a wide variety of models like LLaMA 3, GPT-4, Claude).
- **Auto Git Push**: Automatically commits and pushes generated code.

## Installation

You can install MIG via pip:

```bash
pip install -e .
```

## Setup

1. **Environment Variables**: Create a `.env` file in the root of your repository where you want to run MIG.
```env
# Use Gemini (Recommended)
GEMINI_API_KEY=your_gemini_api_key_here
# GEMINI_MODEL=gemini-2.5-flash-lite

# Or use Groq
GROQ_API_KEY=your_groq_api_key_here
# GROQ_MODEL=llama-3.1-8b-instant

# Or use OpenRouter
OPENROUTER_API_KEY=your_openrouter_api_key_here
# OPENROUTER_MODEL=openrouter/free
```

2. **Git Repository**: Make sure you are running MIG inside a cloned git repository where you have push access.

3. **Context.md**: Modify `Context.md` to change MIG's personality and what kind of code it should generate.

## Usage

To run MIG manually:
```bash
mig run
```

To see cron installation instructions:
```bash
mig install-cron
```

## How it works

1. MIG reads `Context.md` to understand its persona.
2. It fetches its past memory from a local SQLite database (`mig_memory.sqlite`).
3. It asks the configured LLM to generate a new, unique contribution.
4. It saves the generated file, commits it, and pushes it to your remote git repository.
