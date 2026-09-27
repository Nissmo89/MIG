package memory

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSQLiteMemory(t *testing.T) {
	tempDir := t.TempDir()
	dbPath := filepath.Join(tempDir, "test_memory.sqlite")

	store, err := Open(dbPath)
	if err != nil {
		t.Fatalf("failed to open sqlite store: %v", err)
	}
	defer store.Close()

	// Verify file was created
	if _, err := os.Stat(dbPath); os.IsNotExist(err) {
		t.Fatalf("sqlite database file was not created on disk")
	}

	// Insert contribution
	err = store.SaveContribution("c/bit_tricks.c", "feat(c): bitwise manipulation tricks", "c", "#include <stdio.h>")
	if err != nil {
		t.Fatalf("failed to save contribution: %v", err)
	}

	// Retrieve contributions
	recent, err := store.GetRecentContributions(10)
	if err != nil {
		t.Fatalf("failed to get recent contributions: %v", err)
	}

	if len(recent) != 1 {
		t.Fatalf("expected 1 contribution, got %d", len(recent))
	}

	if recent[0].Filename != "c/bit_tricks.c" {
		t.Errorf("expected filename 'c/bit_tricks.c', got %s", recent[0].Filename)
	}
	if recent[0].Language != "c" {
		t.Errorf("expected language 'c', got %s", recent[0].Language)
	}
}
