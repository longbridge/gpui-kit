package main

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"
)

// Default timeout duration for operations
const timeout = 5 * time.Second

var (
	instanceCount int
	mu           sync.RWMutex
)

/**
 * HelloWorld represents a greeter with configuration options
 * Contains:
 * - name: String identifier for the greeter
 * - createdAt: Timestamp when instance was created
 * - options: Map of configuration options
 */
type HelloWorld struct {
	name      string
	createdAt time.Time
	options   map[string]interface{}
}

type Config struct {
	Timeout  time.Duration `json:"timeout"`
	Retries  int          `json:"retries"`
	Debug    bool         `json:"debug"`
}

func NewHelloWorld(name string) *HelloWorld {
	mu.Lock()
	instanceCount++
	mu.Unlock()
	return &HelloWorld{
		name:      name,
		createdAt: time.Now(),
		options:   make(map[string]interface{}),
	}
}

func (h *HelloWorld) Greet(ctx context.Context, names ...string) error {
	for _, name := range names {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
			fmt.Printf("Hello, %s!\n", name)
		}
	}
	return nil
}

func (h *HelloWorld) Configure(cfg Config) {
	h.options["timeout"] = cfg.Timeout
	h.options["retries"] = cfg.Retries
	h.options["debug"] = cfg.Debug
}

func (h *HelloWorld) generateReport() string {
	data, _ := json.MarshalIndent(h.options, "", "  ")
	return fmt.Sprintf(`
		HelloWorld Report
		================
		Name: %s
		Created: %s
		Options: %s
	`, h.name, h.createdAt.Format(time.RFC3339), string(data))
}

func main() {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	greeter := NewHelloWorld("Go")
	greeter.Configure(Config{
		Timeout: timeout,
		Retries: 3,
		Debug:   true,
	})

	if err := greeter.Greet(ctx, "Alice", "Bob"); err != nil {
		fmt.Printf("Error greeting: %v\n", err)
	}
	fmt.Println(greeter.generateReport())
}
