package cloud

import (
    "time"
)

type CloudFile struct {
    ID           string
    Name         string
    Path         string
    Hash         string
    Size         int64
    ModifiedTime time.Time
    Provider     string
}

type Provider interface {
    ListFiles(path string) ([]CloudFile, error)
    RenameFile(file *CloudFile, newName string) error
    DeleteFile(file *CloudFile) error
    Name() string
}
