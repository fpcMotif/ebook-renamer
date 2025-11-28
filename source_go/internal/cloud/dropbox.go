package cloud

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
    "path/filepath"
    "time"
)

type DropboxProvider struct {
    AccessToken string
    Client      *http.Client
}

func NewDropboxProvider(token string) *DropboxProvider {
    return &DropboxProvider{
        AccessToken: token,
        Client:      &http.Client{},
    }
}

func (p *DropboxProvider) Name() string {
    return "dropbox"
}

func (p *DropboxProvider) request(endpoint string, body interface{}) (map[string]interface{}, error) {
    jsonBody, err := json.Marshal(body)
    if err != nil {
        return nil, err
    }

    req, err := http.NewRequest("POST", "https://api.dropboxapi.com/2/files/"+endpoint, bytes.NewBuffer(jsonBody))
    if err != nil {
        return nil, err
    }

    req.Header.Set("Authorization", "Bearer "+p.AccessToken)
    req.Header.Set("Content-Type", "application/json")

    resp, err := p.Client.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return nil, fmt.Errorf("dropbox api error: %s", resp.Status)
    }

    var result map[string]interface{}
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return nil, err
    }
    return result, nil
}

func (p *DropboxProvider) ListFiles(path string) ([]CloudFile, error) {
    var files []CloudFile
    hasMore := true
    var cursor string

    if path == "." || path == "/" {
        path = ""
    }

    for hasMore {
        var body map[string]interface{}
        endpoint := "list_folder"

        if cursor != "" {
            endpoint = "list_folder/continue"
            body = map[string]interface{}{"cursor": cursor}
        } else {
            body = map[string]interface{}{
                "path":                                path,
                "recursive":                           true,
                "include_media_info":                  false,
                "include_deleted":                     false,
                "include_has_explicit_shared_members": false,
            }
        }

        res, err := p.request(endpoint, body)
        if err != nil {
            return nil, err
        }

        if entries, ok := res["entries"].([]interface{}); ok {
            for _, e := range entries {
                entry := e.(map[string]interface{})
                if tag, ok := entry[".tag"].(string); ok && tag == "file" {
                    name := entry["name"].(string)
                    pathDisplay := entry["path_display"].(string)
                    id := entry["id"].(string)
                    size := int64(entry["size"].(float64))
                    hash := ""
                    if h, ok := entry["content_hash"].(string); ok {
                        hash = h
                    }

                    // Client modified
                    modTime := time.Now()
                    if cm, ok := entry["client_modified"].(string); ok {
                        if t, err := time.Parse(time.RFC3339, cm+"Z"); err == nil { // Dropbox sends UTC without Z sometimes? Docs say ISO8601
                            modTime = t
                        }
                    }

                    files = append(files, CloudFile{
                        ID:           id,
                        Name:         name,
                        Path:         pathDisplay,
                        Hash:         hash,
                        Size:         size,
                        ModifiedTime: modTime,
                        Provider:     "dropbox",
                    })
                }
            }
        }

        if hm, ok := res["has_more"].(bool); ok {
            hasMore = hm
        } else {
            hasMore = false
        }

        if c, ok := res["cursor"].(string); ok {
            cursor = c
        }
    }

    return files, nil
}

func (p *DropboxProvider) RenameFile(file *CloudFile, newName string) error {
    parent := filepath.Dir(file.Path)
    newPath := filepath.Join(parent, newName)
    // Ensure slash
    if newPath[0] != '/' {
        newPath = "/" + newPath
    }

    body := map[string]interface{}{
        "from_path":  file.Path,
        "to_path":    newPath,
        "autorename": false,
    }

    _, err := p.request("move_v2", body)
    return err
}

func (p *DropboxProvider) DeleteFile(file *CloudFile) error {
    body := map[string]interface{}{
        "path": file.Path,
    }
    _, err := p.request("delete_v2", body)
    return err
}
