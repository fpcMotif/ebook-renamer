package config

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
	"github.com/mitchellh/go-homedir"
)

type Config struct {
	DefaultDir            *string  `toml:"default_dir"`
	CloudDetectionEnabled *bool    `toml:"cloud_detection"`
	InteractiveModeOn     *bool    `toml:"interactive_mode"`
	DryRunDefaultOn       *bool    `toml:"dry_run_default"`
	IgnoredPatternsList   []string `toml:"ignored_patterns"`
	HashCloudFilesOn      *bool    `toml:"hash_cloud_files"`
	FetchArxivOn          *bool    `toml:"fetch_arxiv"`
	ExtractMetadataOn     *bool    `toml:"extract_metadata"`
	CleanupDownloadsOn    *bool    `toml:"cleanup_downloads"`
}

func Load() (*Config, error) {
	configPath, err := getConfigPath()
	if err != nil {
		return nil, err
	}

	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return &Config{}, nil
	}

	var config Config
	if _, err := toml.DecodeFile(configPath, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %w", err)
	}

	return &config, nil
}

func (c *Config) Save() (string, error) {
	configPath, err := getConfigPath()
	if err != nil {
		return "", err
	}

	if err := os.MkdirAll(filepath.Dir(configPath), 0755); err != nil {
		return "", fmt.Errorf("failed to create config directory: %w", err)
	}

	file, err := os.Create(configPath)
	if err != nil {
		return "", fmt.Errorf("failed to create config file: %w", err)
	}
	defer file.Close()

	encoder := toml.NewEncoder(file)
	encoder.Indent = ""
	if err := encoder.Encode(c); err != nil {
		return "", fmt.Errorf("failed to write config: %w", err)
	}

	return configPath, nil
}

func (c *Config) CloudDetection() bool {
	if c.CloudDetectionEnabled == nil {
		return true
	}
	return *c.CloudDetectionEnabled
}

func (c *Config) InteractiveMode() bool {
	if c.InteractiveModeOn == nil {
		return false
	}
	return *c.InteractiveModeOn
}

func (c *Config) DryRunDefault() bool {
	if c.DryRunDefaultOn == nil {
		return false
	}
	return *c.DryRunDefaultOn
}

func (c *Config) HashCloudFiles() bool {
	if c.HashCloudFilesOn == nil {
		return true
	}
	return *c.HashCloudFilesOn
}

func (c *Config) FetchArxiv() bool {
	if c.FetchArxivOn == nil {
		return false
	}
	return *c.FetchArxivOn
}

func (c *Config) ExtractMetadata() bool {
	if c.ExtractMetadataOn == nil {
		return false
	}
	return *c.ExtractMetadataOn
}

func (c *Config) CleanupDownloads() bool {
	if c.CleanupDownloadsOn == nil {
		return false
	}
	return *c.CleanupDownloadsOn
}

func (c *Config) IgnoredPatterns() []string {
	return c.IgnoredPatternsList
}

func (c *Config) GetDefaultDir() string {
	if c.DefaultDir == nil {
		return ""
	}
	return *c.DefaultDir
}

func getConfigPath() (string, error) {
	if xdgConfigHome := os.Getenv("XDG_CONFIG_HOME"); xdgConfigHome != "" {
		return filepath.Join(xdgConfigHome, "ebook-renamer", "config.toml"), nil
	}

	home, err := homedir.Dir()
	if err != nil {
		return "", fmt.Errorf("could not determine home directory: %w", err)
	}

	return filepath.Join(home, ".config", "ebook-renamer", "config.toml"), nil
}
