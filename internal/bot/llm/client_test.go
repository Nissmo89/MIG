package llm

import (
	"testing"
)

func TestCleanJSON_Plain(t *testing.T) {
	input := `{"filename":"python/solution.py","commit_message":"feat: add solution","code":"print('hello')"}`
	payload, err := CleanJSON(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if payload.Filename != "python/solution.py" {
		t.Errorf("expected filename python/solution.py, got %s", payload.Filename)
	}
	if payload.CommitMessage != "feat: add solution" {
		t.Errorf("expected commit message 'feat: add solution', got %s", payload.CommitMessage)
	}
	if payload.Code != "print('hello')" {
		t.Errorf("expected code 'print(\\'hello\\')', got %s", payload.Code)
	}
}

func TestCleanJSON_MarkdownFence(t *testing.T) {
	input := "```json\n{\n  \"filename\": \"rust/fast_fib.rs\",\n  \"commit_message\": \"feat(rust): fast matrix fibonacci\",\n  \"code\": \"fn main() {}\"\n}\n```"
	payload, err := CleanJSON(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if payload.Filename != "rust/fast_fib.rs" {
		t.Errorf("expected rust/fast_fib.rs, got %s", payload.Filename)
	}
}

func TestCleanJSON_ConversationalPadding(t *testing.T) {
	input := "Sure! Here is the generated code snippet for you:\n\n```json\n{\"filename\": \"go/trie.go\", \"commit_message\": \"feat: implement trie\", \"code\": \"package main\"}\n```\nHope you like it!"
	payload, err := CleanJSON(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if payload.Filename != "go/trie.go" {
		t.Errorf("expected go/trie.go, got %s", payload.Filename)
	}
}

func TestCleanJSON_Invalid(t *testing.T) {
	input := "This is not json at all."
	_, err := CleanJSON(input)
	if err == nil {
		t.Fatalf("expected error for invalid json, got nil")
	}
}
