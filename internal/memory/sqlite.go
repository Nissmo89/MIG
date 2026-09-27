package memory

import (
	"database/sql"
	"fmt"
	"time"

	_ "modernc.org/sqlite"
)

type Contribution struct {
	ID            int64     `json:"id"`
	Filename      string    `json:"filename"`
	CommitMessage string    `json:"commit_message"`
	Language      string    `json:"language"`
	Code          string    `json:"code"`
	CreatedAt     time.Time `json:"created_at"`
}

type Store struct {
	db *sql.DB
}

func Open(dbPath string) (*Store, error) {
	if dbPath == "" {
		dbPath = "mig_memory.sqlite"
	}

	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open sqlite database: %w", err)
	}

	schema := `
	CREATE TABLE IF NOT EXISTS contributions (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		filename TEXT NOT NULL,
		commit_message TEXT NOT NULL,
		language TEXT NOT NULL,
		code TEXT NOT NULL,
		created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
	);
	`
	if _, err := db.Exec(schema); err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to create contributions table: %w", err)
	}

	return &Store{db: db}, nil
}

func (s *Store) Close() error {
	if s.db != nil {
		return s.db.Close()
	}
	return nil
}

func (s *Store) SaveContribution(filename, commitMessage, language, code string) error {
	query := `INSERT INTO contributions (filename, commit_message, language, code, created_at) VALUES (?, ?, ?, ?, ?)`
	_, err := s.db.Exec(query, filename, commitMessage, language, code, time.Now())
	return err
}

func (s *Store) GetRecentContributions(limit int) ([]Contribution, error) {
	if limit <= 0 {
		limit = 10
	}

	query := `SELECT id, filename, commit_message, language, code, created_at FROM contributions ORDER BY id DESC LIMIT ?`
	rows, err := s.db.Query(query, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []Contribution
	for rows.Next() {
		var c Contribution
		if err := rows.Scan(&c.ID, &c.Filename, &c.CommitMessage, &c.Language, &c.Code, &c.CreatedAt); err != nil {
			return nil, err
		}
		results = append(results, c)
	}

	return results, rows.Err()
}
