package llm

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"time"
)

type LLMResponse struct {
	RawContent    string
	Filename      string
	CommitMessage string
	Code          string
	InputTokens   int64
	OutputTokens  int64
	TotalTokens   int64
	ModelName     string
}

type Client interface {
	Generate(ctx context.Context, systemPrompt, userPrompt string) (*LLMResponse, error)
	GetModelName() string
	GetProviderName() string
}

type GenerationPayload struct {
	Filename      string `json:"filename"`
	CommitMessage string `json:"commit_message"`
	Code          string `json:"code"`
}

// CleanJSON extracts and unmarshals JSON from an LLM string
func CleanJSON(raw string) (*GenerationPayload, error) {
	content := strings.TrimSpace(raw)

	// Remove markdown fences
	if strings.HasPrefix(content, "```json") {
		content = strings.TrimPrefix(content, "```json")
	} else if strings.HasPrefix(content, "```") {
		content = strings.TrimPrefix(content, "```")
	}

	if strings.HasSuffix(content, "```") {
		content = strings.TrimSuffix(content, "```")
	}
	content = strings.TrimSpace(content)

	// In case there is conversational text before or after JSON
	startIdx := strings.Index(content, "{")
	endIdx := strings.LastIndex(content, "}")
	if startIdx != -1 && endIdx != -1 && endIdx > startIdx {
		content = content[startIdx : endIdx+1]
	}

	var payload GenerationPayload
	if err := json.Unmarshal([]byte(content), &payload); err != nil {
		return nil, fmt.Errorf("failed to parse JSON response: %w (content was: %s)", err, content)
	}

	if payload.Filename == "" || payload.Code == "" {
		return nil, fmt.Errorf("invalid payload: missing filename or code in response: %s", content)
	}

	return &payload, nil
}

// --- Gemini Client ---

type GeminiClient struct {
	apiKey     string
	model      string
	httpClient *http.Client
}

func NewGeminiClient(apiKey, model string) *GeminiClient {
	if model == "" {
		model = "gemini-2.5-flash-lite"
	}
	return &GeminiClient{
		apiKey: apiKey,
		model:  model,
		httpClient: &http.Client{
			Timeout: 90 * time.Second,
		},
	}
}

func (c *GeminiClient) GetModelName() string    { return c.model }
func (c *GeminiClient) GetProviderName() string { return "Gemini" }

func (c *GeminiClient) Generate(ctx context.Context, systemPrompt, userPrompt string) (*LLMResponse, error) {
	url := fmt.Sprintf("https://generativelanguage.googleapis.com/v1beta/models/%s:generateContent?key=%s", c.model, c.apiKey)

	promptText := fmt.Sprintf("System Instructions:\n%s\n\nTask:\n%s", systemPrompt, userPrompt)

	reqBody := map[string]interface{}{
		"contents": []map[string]interface{}{
			{
				"role": "user",
				"parts": []map[string]interface{}{
					{"text": promptText},
				},
			},
		},
		"generationConfig": map[string]interface{}{
			"temperature": 0.7,
		},
	}

	jsonData, err := json.Marshal(reqBody)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewBuffer(jsonData))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("gemini request error: %w", err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("gemini read response error: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("gemini API returned status %d: %s", resp.StatusCode, string(bodyBytes))
	}

	var geminiResp struct {
		Candidates []struct {
			Content struct {
				Parts []struct {
					Text string `json:"text"`
				} `json:"parts"`
			} `json:"content"`
		} `json:"candidates"`
		UsageMetadata struct {
			PromptTokenCount     int64 `json:"promptTokenCount"`
			CandidatesTokenCount int64 `json:"candidatesTokenCount"`
			TotalTokenCount      int64 `json:"totalTokenCount"`
		} `json:"usageMetadata"`
	}

	if err := json.Unmarshal(bodyBytes, &geminiResp); err != nil {
		return nil, fmt.Errorf("gemini response decode error: %w", err)
	}

	if len(geminiResp.Candidates) == 0 || len(geminiResp.Candidates[0].Content.Parts) == 0 {
		return nil, errors.New("gemini returned no content candidates")
	}

	rawText := geminiResp.Candidates[0].Content.Parts[0].Text
	payload, err := CleanJSON(rawText)
	if err != nil {
		return nil, err
	}

	totTokens := geminiResp.UsageMetadata.TotalTokenCount
	inTokens := geminiResp.UsageMetadata.PromptTokenCount
	outTokens := geminiResp.UsageMetadata.CandidatesTokenCount
	if totTokens == 0 {
		totTokens = int64((len(promptText) + len(rawText)) / 4)
		inTokens = totTokens / 2
		outTokens = totTokens / 2
	}

	return &LLMResponse{
		RawContent:    rawText,
		Filename:      payload.Filename,
		CommitMessage: payload.CommitMessage,
		Code:          payload.Code,
		InputTokens:   inTokens,
		OutputTokens:  outTokens,
		TotalTokens:   totTokens,
		ModelName:     c.model,
	}, nil
}

// --- OpenAI-Compatible Client (Groq and OpenRouter) ---

type OpenAICompatibleClient struct {
	providerName string
	apiKey       string
	baseURL      string
	model        string
	httpClient   *http.Client
}

func NewGroqClient(apiKey, model string) *OpenAICompatibleClient {
	if model == "" {
		model = "llama-3.1-8b-instant"
	}
	return &OpenAICompatibleClient{
		providerName: "Groq",
		apiKey:       apiKey,
		baseURL:      "https://api.groq.com/openai/v1",
		model:        model,
		httpClient: &http.Client{
			Timeout: 90 * time.Second,
		},
	}
}

func NewOpenRouterClient(apiKey, model string) *OpenAICompatibleClient {
	if model == "" {
		model = "openrouter/free"
	}
	return &OpenAICompatibleClient{
		providerName: "OpenRouter",
		apiKey:       apiKey,
		baseURL:      "https://openrouter.ai/api/v1",
		model:        model,
		httpClient: &http.Client{
			Timeout: 90 * time.Second,
		},
	}
}

func (c *OpenAICompatibleClient) GetModelName() string    { return c.model }
func (c *OpenAICompatibleClient) GetProviderName() string { return c.providerName }

func (c *OpenAICompatibleClient) Generate(ctx context.Context, systemPrompt, userPrompt string) (*LLMResponse, error) {
	url := fmt.Sprintf("%s/chat/completions", c.baseURL)

	reqBody := map[string]interface{}{
		"model": c.model,
		"messages": []map[string]string{
			{"role": "system", "content": systemPrompt},
			{"role": "user", "content": userPrompt},
		},
		"temperature": 0.7,
	}

	jsonData, err := json.Marshal(reqBody)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewBuffer(jsonData))
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", c.apiKey))
	if c.providerName == "OpenRouter" {
		req.Header.Set("HTTP-Referer", "https://github.com/Nissmo89/MIG")
		req.Header.Set("X-Title", "MIG Auto Bot")
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("%s request error: %w", c.providerName, err)
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("%s read response error: %w", c.providerName, err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("%s API returned status %d: %s", c.providerName, resp.StatusCode, string(bodyBytes))
	}

	var chatResp struct {
		Model   string `json:"model"`
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		} `json:"choices"`
		Usage struct {
			PromptTokens     int64 `json:"prompt_tokens"`
			CompletionTokens int64 `json:"completion_tokens"`
			TotalTokens      int64 `json:"total_tokens"`
		} `json:"usage"`
	}

	if err := json.Unmarshal(bodyBytes, &chatResp); err != nil {
		return nil, fmt.Errorf("%s response decode error: %w", c.providerName, err)
	}

	if len(chatResp.Choices) == 0 {
		return nil, fmt.Errorf("%s returned no message choices", c.providerName)
	}

	rawText := chatResp.Choices[0].Message.Content
	payload, err := CleanJSON(rawText)
	if err != nil {
		return nil, err
	}

	totTokens := chatResp.Usage.TotalTokens
	inTokens := chatResp.Usage.PromptTokens
	outTokens := chatResp.Usage.CompletionTokens
	if totTokens == 0 {
		totTokens = int64((len(systemPrompt) + len(userPrompt) + len(rawText)) / 4)
		inTokens = totTokens / 2
		outTokens = totTokens / 2
	}

	respModel := chatResp.Model
	if respModel == "" {
		respModel = c.model
	}

	return &LLMResponse{
		RawContent:    rawText,
		Filename:      payload.Filename,
		CommitMessage: payload.CommitMessage,
		Code:          payload.Code,
		InputTokens:   inTokens,
		OutputTokens:  outTokens,
		TotalTokens:   totTokens,
		ModelName:     respModel,
	}, nil
}

// GetClient initializes the appropriate LLM client based on available environment variables.
// Priority: Gemini -> Groq -> OpenRouter
func GetClient() (Client, error) {
	geminiKey := os.Getenv("GEMINI_API_KEY")
	if geminiKey == "" {
		geminiKey = os.Getenv("GOOGLE_API_KEY")
	}

	if geminiKey != "" {
		model := os.Getenv("GEMINI_MODEL")
		if model == "" {
			model = "gemini-2.5-flash-lite"
		}
		return NewGeminiClient(geminiKey, model), nil
	}

	groqKey := os.Getenv("GROQ_API_KEY")
	if groqKey != "" {
		model := os.Getenv("GROQ_MODEL")
		if model == "" {
			model = "llama-3.1-8b-instant"
		}
		return NewGroqClient(groqKey, model), nil
	}

	openRouterKey := os.Getenv("OPENROUTER_API_KEY")
	if openRouterKey != "" {
		model := os.Getenv("OPENROUTER_MODEL")
		if model == "" {
			model = "openrouter/free"
		}
		return NewOpenRouterClient(openRouterKey, model), nil
	}

	return nil, errors.New("GEMINI_API_KEY, GROQ_API_KEY, or OPENROUTER_API_KEY must be set in the environment")
}

// ProviderStatus represents status information for an LLM provider
type ProviderStatus struct {
	Provider   string `json:"provider"`
	Configured bool   `json:"configured"`
	MaskedKey  string `json:"masked_key"`
	Model      string `json:"model"`
	IsActive   bool   `json:"is_active"`
}

// GetProvidersStatus returns status for all supported providers
func GetProvidersStatus() []ProviderStatus {
	mask := func(key string) string {
		if key == "" {
			return "None"
		}
		if len(key) > 10 {
			return fmt.Sprintf("%s...%s", key[:4], key[len(key)-4:])
		}
		return "******"
	}

	geminiKey := os.Getenv("GEMINI_API_KEY")
	if geminiKey == "" {
		geminiKey = os.Getenv("GOOGLE_API_KEY")
	}
	groqKey := os.Getenv("GROQ_API_KEY")
	openRouterKey := os.Getenv("OPENROUTER_API_KEY")

	geminiActive := geminiKey != ""
	groqActive := !geminiActive && groqKey != ""
	openRouterActive := !geminiActive && !groqActive && openRouterKey != ""

	geminiModel := os.Getenv("GEMINI_MODEL")
	if geminiModel == "" {
		geminiModel = "gemini-2.5-flash-lite"
	}
	groqModel := os.Getenv("GROQ_MODEL")
	if groqModel == "" {
		groqModel = "llama-3.1-8b-instant"
	}
	openRouterModel := os.Getenv("OPENROUTER_MODEL")
	if openRouterModel == "" {
		openRouterModel = "openrouter/free"
	}

	return []ProviderStatus{
		{
			Provider:   "Google Gemini",
			Configured: geminiKey != "",
			MaskedKey:  mask(geminiKey),
			Model:      geminiModel,
			IsActive:   geminiActive,
		},
		{
			Provider:   "Groq",
			Configured: groqKey != "",
			MaskedKey:  mask(groqKey),
			Model:      groqModel,
			IsActive:   groqActive,
		},
		{
			Provider:   "OpenRouter",
			Configured: openRouterKey != "",
			MaskedKey:  mask(openRouterKey),
			Model:      openRouterModel,
			IsActive:   openRouterActive,
		},
	}
}
