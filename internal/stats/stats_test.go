package stats

import (
	"os"
	"path/filepath"
	"testing"
)

func TestCalculateCost(t *testing.T) {
	cost := CalculateCost("llama-3.1-8b-instant", 1000, 1000)
	expected := (1000.0/1_000_000.0)*0.05 + (1000.0/1_000_000.0)*0.08
	diff := cost - expected
	if diff < -0.0000001 || diff > 0.0000001 {
		t.Errorf("expected cost %f, got %f", expected, cost)
	}
}

func TestLanguageCounts(t *testing.T) {
	tempDir := t.TempDir()

	counts, err := GetLanguageCounts(tempDir)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(counts) != 0 {
		t.Errorf("expected empty counts, got %v", counts)
	}

	if err := IncrementLanguageCount(tempDir, "rust"); err != nil {
		t.Fatalf("failed to increment language count: %v", err)
	}
	if err := IncrementLanguageCount(tempDir, "rust"); err != nil {
		t.Fatalf("failed to increment language count: %v", err)
	}
	if err := IncrementLanguageCount(tempDir, "go"); err != nil {
		t.Fatalf("failed to increment language count: %v", err)
	}

	counts, err = GetLanguageCounts(tempDir)
	if err != nil {
		t.Fatalf("failed to read counts: %v", err)
	}

	if counts["rust"] != 2 {
		t.Errorf("expected 2 rust files, got %d", counts["rust"])
	}
	if counts["go"] != 1 {
		t.Errorf("expected 1 go file, got %d", counts["go"])
	}

	// Verify file was written to disk
	if _, err := os.Stat(filepath.Join(tempDir, "count.json")); os.IsNotExist(err) {
		t.Errorf("count.json not found on disk")
	}
}
