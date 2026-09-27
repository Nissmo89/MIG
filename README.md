# MIG (Make It Green)

MIG is an autonomous auto-contributing CLI daemon that keeps your GitHub contribution graph active and green by generating clever DSA code snippets, algorithms, and tricks across multiple languages daily.

**100% Standalone Go CLI Binary** • **Zero Runtime Dependencies** (No Python, No CGO, No Node.js, No separate TUI binary).

---

## Features

- **Zero External Dependencies**: Compiles to a single standalone binary with `CGO_ENABLED=0`. Runs on Windows, macOS, and Linux without needing any C compiler or Python runtime.
- **Embedded Web Dashboard**: Modern, dark-themed responsive single-page dashboard embedded directly inside the Go binary (`//go:embed`). No web server or Node runtime required.
- **Multi-Provider LLM Integration**: Lightweight native REST clients for:
  - **Google Gemini** (`gemini-2.5-flash-lite`, `gemini-1.5-flash`, etc.)
  - **Groq** (`llama-3.1-8b-instant`, `llama-3.1-70b-versatile`, etc.)
  - **OpenRouter** (`openrouter/free`, `anthropic/claude-3.5-sonnet`, `openai/gpt-4o`, etc.)
- **CGO-Free SQLite Memory**: Powered by `modernc.org/sqlite` to store and remember past contributions in `mig_memory.sqlite`, ensuring the bot never repeats itself.
- **Local & Global Analytics**: Tracks token consumption, latency, and costs across models in `~/.mig/stats.json` and `~/.mig/groq_analytics.json`, with language breakdown in `count.json`.
- **Autonomous Git Push**: Automatically stages files, generates conventional commit messages, and pushes to remote.

---

## Building from Source

### Prerequisites
- Go 1.22 or later

### Build Binary
```bash
# Build standalone binary (mig.exe on Windows, mig on Unix)
CGO_ENABLED=0 go build -o mig .
```

---

## Setup

1. **Environment Variables**: Create a `.env` file in the root of your target repository:
```env
# Google Gemini (Recommended)
GEMINI_API_KEY=your_gemini_api_key_here
# GEMINI_MODEL=gemini-2.5-flash-lite

# Or Groq
GROQ_API_KEY=your_groq_api_key_here
# GROQ_MODEL=llama-3.1-8b-instant

# Or OpenRouter
OPENROUTER_API_KEY=your_openrouter_api_key_here
# OPENROUTER_MODEL=openrouter/free
```

2. **Initialize Repository**:
```bash
./mig init
```
This initializes git (if missing), creates default `Context.md`, and registers the project in `~/.mig/projects.json`.

3. **Personality Customization**:
Edit `Context.md` in your repository to customize MIG's personality, preferred languages, and code styling rules.

---

## Usage

### Run Contribution
Generate, write, commit, and push a new contribution:
```bash
./mig run
```

### Launch Web Dashboard
Start the local embedded web dashboard and open it automatically in your browser:
```bash
./mig dashboard
# or with alias
./mig web

# Custom port
./mig dashboard --port 8080
```
The dashboard provides:
- Real-time token analytics and aggregated cost metrics
- Active LLM provider statuses
- Managed repository tracking overview
- Visual language breakdown
- Recent commits activity feed
- An interactive **"Trigger Run"** button for on-demand contributions

### Automation & Scheduling
To print setup instructions for daily automated execution:
```bash
./mig install-cron
```
- **Linux / macOS**: Cron schedule via `crontab -e`
- **Windows**: Windows Task Scheduler via `schtasks`

---

## Architecture

```
mig/
├── cmd/               # Cobra CLI commands (root, init, run, cron, dashboard)
├── internal/
│   ├── bot/           # Autonomous contribution runner & state orchestrator
│   │   └── llm/       # REST clients for Gemini, Groq, and OpenRouter
│   ├── git/           # Git lifecycle execution (os/exec)
│   ├── memory/        # Pure Go SQLite operations (modernc.org/sqlite)
│   ├── server/        # Local HTTP server & API endpoints
│   └── stats/         # Global & project telemetry (~/.mig/*.json)
├── web/               # Embedded web dashboard (HTML + Tailwind CSS + JS)
├── go.mod
├── go.sum
└── main.go            # Entry point
```

---

## Verification & Testing

Run all unit tests:
```bash
go test ./... -v
```
