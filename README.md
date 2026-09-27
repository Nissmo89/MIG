# MIG (Make It Green)

MIG is an autonomous auto-contributing CLI daemon that keeps your GitHub contribution graph active and green by generating clever DSA code snippets, micro-algorithms, and tricks across multiple languages daily.

**100% Standalone Go CLI Binary** • **Zero Runtime Dependencies** (No Python, No CGO, No Node.js, No separate TUI binary).

---

## Table of Contents
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Clone & Switch Branch](#clone--switch-branch)
- [Build Instructions](#build-instructions)
  - [Linux / macOS](#linux--macos)
  - [Windows (PowerShell)](#windows-powershell)
  - [Windows (Command Prompt)](#windows-command-prompt)
  - [Cross-Compilation](#cross-compilation)
- [Configuration](#configuration)
- [Command Reference & Usage](#command-reference--usage)
  - [1. Initialize Project (`mig init`)](#1-initialize-project-mig-init)
  - [2. Run Contribution (`mig run`)](#2-run-contribution-mig-run)
  - [3. Interactive Web Dashboard (`mig dashboard`)](#3-interactive-web-dashboard-mig-dashboard)
  - [4. Setup Daily Scheduling (`mig install-cron`)](#4-setup-daily-scheduling-mig-install-cron)
- [Daily Automation Setup](#daily-automation-setup)
- [Running Tests](#running-tests)
- [Project Architecture](#project-architecture)

---

## Features

- **Zero External Dependencies**: Compiles to a single standalone binary using `CGO_ENABLED=0`. Runs on Windows, macOS, and Linux without requiring a C compiler, Python runtime, or Node.js.
- **Embedded Web Dashboard**: Modern, dark-themed responsive single-page dashboard embedded directly into the binary (`//go:embed`). No external web server needed.
- **Multi-Provider LLM Integration**: Lightweight native REST clients for:
  - **Google Gemini** (`gemini-2.5-flash-lite`, `gemini-1.5-flash`, etc.)
  - **Groq** (`llama-3.1-8b-instant`, `llama-3.1-70b-versatile`, etc.)
  - **OpenRouter** (`openrouter/free`, `anthropic/claude-3.5-sonnet`, `openai/gpt-4o`, etc.)
- **CGO-Free SQLite Memory**: Uses `modernc.org/sqlite` to record past contributions in `mig_memory.sqlite`, ensuring the bot never repeats previously generated algorithms.
- **Local & Global Telemetry**: Tracks token usage, latency, and costs across models in `~/.mig/stats.json` and `~/.mig/groq_analytics.json`, with language breakdowns in `count.json`.
- **Autonomous Git Lifecycle**: Automatically stages generated files, generates conventional commit messages, and pushes to remote repositories.

---

## Prerequisites

### 1. Install Go (v1.22 or later)
- **Linux (Ubuntu/Debian)**:
  ```bash
  sudo apt update && sudo apt install -y golang
  ```
- **Linux (Arch Linux)**:
  ```bash
  sudo pacman -S go
  ```
- **macOS (Homebrew)**:
  ```bash
  brew install go
  ```
- **Windows (winget)**:
  ```powershell
  winget install --id GoLang.Go -e
  ```
- **Or download directly**: [https://go.dev/dl/](https://go.dev/dl/)

Verify installation:
```bash
go version
```

### 2. Git
Make sure `git` is installed and configured with access to push to your remote GitHub repository:
```bash
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

---

## Clone & Switch Branch

To clone the repository and switch to the Go migration branch (`feat/migrate-to-go`):

### Linux / macOS
```bash
# Clone repository
git clone https://github.com/Nissmo89/MIG.git

# Enter project directory
cd MIG

# Fetch all remote branches
git fetch origin

# Switch to the Go implementation branch
git checkout feat/migrate-to-go
```

### Windows (PowerShell or CMD)
```powershell
# Clone repository
git clone https://github.com/Nissmo89/MIG.git

# Enter project directory
cd MIG

# Fetch all remote branches
git fetch origin

# Switch to the Go implementation branch
git checkout feat/migrate-to-go
```

---

## Build Instructions

Because MIG uses pure Go SQLite (`modernc.org/sqlite`), building with `CGO_ENABLED=0` generates a completely static binary that requires no external libraries.

### Linux / macOS
```bash
# Compile standalone binary
CGO_ENABLED=0 go build -ldflags="-s -w" -o mig .

# Make binary executable (if not already)
chmod +x mig

# Verify binary
./mig --help
```

### Windows (PowerShell)
```powershell
# Compile standalone binary
$env:CGO_ENABLED="0"
go build -ldflags="-s -w" -o mig.exe .

# Verify binary
.\mig.exe --help
```

### Windows (Command Prompt)
```cmd
:: Set CGO to 0
set CGO_ENABLED=0

:: Compile standalone binary
go build -ldflags="-s -w" -o mig.exe .

:: Verify binary
mig.exe --help
```

### Cross-Compilation
You can compile for any OS/architecture from your current machine:

```bash
# Build for Linux 64-bit (from Windows or macOS)
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o mig-linux-amd64 .

# Build for Windows 64-bit (from Linux or macOS)
CGO_ENABLED=0 GOOS=windows GOARCH=amd64 go build -ldflags="-s -w" -o mig-windows-amd64.exe .

# Build for macOS Apple Silicon (ARM64)
CGO_ENABLED=0 GOOS=darwin GOARCH=arm64 go build -ldflags="-s -w" -o mig-darwin-arm64 .
```

---

## Configuration

### 1. Environment Variables (`.env`)
Create a `.env` file in the root of your target repository (or in the MIG folder):

```env
# Option 1: Google Gemini (Recommended)
GEMINI_API_KEY=your_gemini_api_key_here
# GEMINI_MODEL=gemini-2.5-flash-lite

# Option 2: Groq
GROQ_API_KEY=your_groq_api_key_here
# GROQ_MODEL=llama-3.1-8b-instant

# Option 3: OpenRouter
OPENROUTER_API_KEY=your_openrouter_api_key_here
# OPENROUTER_MODEL=openrouter/free
```

*Note: MIG automatically selects the provider based on which key is configured in order of priority: Gemini → Groq → OpenRouter.*

### 2. Personality Context (`Context.md`)
MIG reads `Context.md` to define its coding personality, language preferences, and style guidelines. You can edit this file to suit your preferences:
- Select which languages you want (Python, Go, Rust, C, C++, Java, etc.).
- Enforce strict coding styles or specific problem domains (algorithms, utility scripts, data structures).

---

## Command Reference & Usage

### 1. Initialize Project (`mig init`)
Initializes git if needed, creates a default `Context.md`, and registers the project path in `~/.mig/projects.json`.

- **Linux / macOS**:
  ```bash
  ./mig init
  ```
- **Windows**:
  ```powershell
  .\mig.exe init
  ```

### 2. Run Contribution (`mig run`)
Runs the full autonomous flow: loads context, checks SQLite memory, prompts the LLM, writes the code, updates stats, commits, and pushes.

- **Linux / macOS**:
  ```bash
  ./mig run
  ```
- **Windows**:
  ```powershell
  .\mig.exe run
  ```

### 3. Interactive Web Dashboard (`mig dashboard` / `mig web`)
Launches the embedded web dashboard and automatically opens your default browser at `http://localhost:8080`.

- **Linux / macOS**:
  ```bash
  ./mig dashboard
  # or with alias
  ./mig web

  # Custom port
  ./mig dashboard --port 9090
  ```
- **Windows**:
  ```powershell
  .\mig.exe dashboard
  # or with alias
  .\mig.exe web

  # Custom port
  .\mig.exe dashboard --port 9090
  ```

The dashboard includes:
- **Lifetime & Peak Token Metrics**: Aggregated across all runs.
- **Cost Estimation**: Real-time spending tracked from API usage.
- **LLM Provider Cards**: Active model status and masked key verification.
- **Language Distribution**: Dynamic visual progress bars from `count.json`.
- **Managed Repositories**: List of tracked projects from `~/.mig/projects.json`.
- **Recent Git Commits**: Activity feed from tracked repos.
- **Trigger Run Button**: Instant one-click contribution execution with live feedback toasts.

### 4. Setup Daily Scheduling (`mig install-cron`)
Prints exact commands to configure automated execution.

- **Linux / macOS**:
  ```bash
  ./mig install-cron
  ```
- **Windows**:
  ```powershell
  .\mig.exe install-cron
  ```

---

## Daily Automation Setup

### Linux / macOS (Cron)
Open your crontab editor:
```bash
crontab -e
```
Add either of the following entries (replace `/path/to/repo` and `/path/to/mig` with your actual paths):

```cron
# Run automatically on system startup:
@reboot cd /path/to/repo && /path/to/mig run

# Run daily at 9:00 AM:
0 9 * * * cd /path/to/repo && /path/to/mig run
```

### Windows (Task Scheduler)
Open PowerShell **as Administrator** and run:
```powershell
schtasks /create /tn "MIG Daily Contribution" /tr "\"C:\path\to\MIG\mig.exe\" run" /sc daily /st 09:00 /ru "%USERNAME%"
```

To verify the scheduled task:
```powershell
schtasks /query /tn "MIG Daily Contribution"
```

To run the scheduled task manually for testing:
```powershell
schtasks /run /tn "MIG Daily Contribution"
```

---

## Running Tests

To run all unit tests across the packages:

```bash
# Run tests
go test ./... -v

# Run with pure Go CGO_ENABLED=0 mode
CGO_ENABLED=0 go test ./... -v
```

---

## Project Architecture

```
mig/
├── cmd/               # CLI commands implemented with github.com/spf13/cobra
│   ├── root.go        # Root command & .env auto-loader
│   ├── init.go        # 'mig init'
│   ├── run.go         # 'mig run'
│   ├── cron.go        # 'mig install-cron'
│   └── dashboard.go   # 'mig dashboard' & 'mig web'
├── internal/
│   ├── bot/           # Generation flow orchestrator
│   │   └── llm/       # Native net/http REST clients (Gemini, Groq, OpenRouter)
│   ├── git/           # Git operations using os/exec
│   ├── memory/        # CGO-free pure Go SQLite memory (modernc.org/sqlite)
│   ├── server/        # Local HTTP server and JSON REST APIs
│   └── stats/         # Telemetry manager (~/.mig/*.json and count.json)
├── web/               # Embedded SPA frontend
│   ├── embed.go       # //go:embed filesystem bundle
│   └── dist/
│       └── index.html # Responsive dark-themed Catppuccin web dashboard
├── go.mod
├── go.sum
└── main.go            # Binary entrypoint
```

---

## License
MIT License
