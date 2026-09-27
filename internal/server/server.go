package server

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"os/signal"
	"runtime"
	"sync"
	"syscall"
	"time"

	"github.com/Nissmo89/MIG/internal/bot"
	"github.com/Nissmo89/MIG/internal/bot/llm"
	"github.com/Nissmo89/MIG/internal/git"
	"github.com/Nissmo89/MIG/internal/stats"
	"github.com/Nissmo89/MIG/web"
)

var runLock sync.Mutex

func OpenBrowser(url string) {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", url)
	case "darwin":
		cmd = exec.Command("open", url)
	default: // linux, freebsd, etc.
		cmd = exec.Command("xdg-open", url)
	}
	_ = cmd.Start()
}

func StartServer(port int) error {
	if port <= 0 {
		port = 8080
	}

	mux := http.NewServeMux()

	// Embedded Static Files
	fileSystem, err := web.GetFileSystem()
	if err != nil {
		return fmt.Errorf("failed to load embedded filesystem: %w", err)
	}
	fileServer := http.FileServer(fileSystem)
	mux.Handle("/", fileServer)

	// API: Stats & Analytics
	mux.HandleFunc("/api/stats", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		st, _ := stats.GetStats()
		groq, _ := stats.GetGroqAnalytics()
		_ = json.NewEncoder(w).Encode(map[string]interface{}{
			"stats": st,
			"groq":  groq,
		})
	})

	// API: LLM Providers Status
	mux.HandleFunc("/api/providers", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		providers := llm.GetProvidersStatus()
		_ = json.NewEncoder(w).Encode(providers)
	})

	// API: Tracked Projects
	mux.HandleFunc("/api/projects", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		projects, _ := stats.GetProjects()
		_ = json.NewEncoder(w).Encode(projects)
	})

	// API: Language Breakdown
	mux.HandleFunc("/api/languages", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		counts, _ := stats.GetLanguageCounts(".")
		_ = json.NewEncoder(w).Encode(counts)
	})

	// API: Recent Commits Across Projects
	mux.HandleFunc("/api/commits", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		projects, _ := stats.GetProjects()
		if len(projects) == 0 {
			if cwd, err := os.Getwd(); err == nil {
				projects = append(projects, cwd)
			}
		}

		var allCommits []string
		for _, p := range projects {
			if git.IsRepo(p) {
				if commits, err := git.GetRecentCommits(p, 3); err == nil {
					allCommits = append(allCommits, commits...)
				}
			}
		}
		_ = json.NewEncoder(w).Encode(allCommits)
	})

	// API: Trigger Bot Run
	mux.HandleFunc("/api/run", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		if r.Method != http.MethodPost {
			http.Error(w, `{"error":"Method not allowed"}`, http.StatusMethodNotAllowed)
			return
		}

		if !runLock.TryLock() {
			w.WriteHeader(http.StatusConflict)
			_ = json.NewEncoder(w).Encode(map[string]string{
				"status": "busy",
				"error":  "A bot run is already in progress",
			})
			return
		}
		defer runLock.Unlock()

		result, err := bot.RunBot()
		if err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			_ = json.NewEncoder(w).Encode(map[string]interface{}{
				"status": "error",
				"error":  err.Error(),
			})
			return
		}

		_ = json.NewEncoder(w).Encode(map[string]interface{}{
			"status": "success",
			"result": result,
		})
	})

	addr := fmt.Sprintf(":%d", port)
	server := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	url := fmt.Sprintf("http://localhost:%d", port)
	fmt.Printf("\n🚀 MIG Dashboard running at %s\n", url)
	fmt.Println("Press Ctrl+C to stop the dashboard server.")

	// Auto-launch browser
	go func() {
		time.Sleep(200 * time.Millisecond)
		OpenBrowser(url)
	}()

	// Graceful shutdown handling
	stopChan := make(chan os.Signal, 1)
	signal.Notify(stopChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-stopChan
		fmt.Println("\nShutting down MIG Dashboard...")
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		_ = server.Shutdown(ctx)
	}()

	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("server error: %w", err)
	}

	return nil
}
