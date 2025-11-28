package cloud

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
    "time"
)

type GDriveProvider struct {
    AccessToken string
    Client      *http.Client
}

func NewGDriveProvider(token string) *GDriveProvider {
    return &GDriveProvider{
        AccessToken: token,
        Client:      &http.Client{},
    }
}

func (p *GDriveProvider) Name() string {
    return "gdrive"
}

func (p *GDriveProvider) ListFiles(folderID string) ([]CloudFile, error) {
    if folderID == "." || folderID == "/" {
        folderID = "root"
    }

    var files []CloudFile
    var pageToken string

    for {
        url := fmt.Sprintf("https://www.googleapis.com/drive/v3/files?q='%s' in parents and trashed = false&fields=nextPageToken,files(id,name,size,md5Checksum,modifiedTime)&pageSize=1000", folderID)
        if pageToken != "" {
            url += "&pageToken=" + pageToken
        }

        req, _ := http.NewRequest("GET", url, nil)
        req.Header.Set("Authorization", "Bearer "+p.AccessToken)

        resp, err := p.Client.Do(req)
        if err != nil {
            return nil, err
        }
        defer resp.Body.Close()

        if resp.StatusCode != http.StatusOK {
            return nil, fmt.Errorf("gdrive api error: %s", resp.Status)
        }

        var result map[string]interface{}
        if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
            return nil, err
        }

        if items, ok := result["files"].([]interface{}); ok {
            for _, item := range items {
                f := item.(map[string]interface{})
                id := f["id"].(string)
                name := f["name"].(string)
                sizeStr, _ := f["size"].(string) // Google API returns size as string for int64 safety
                var size int64
                fmt.Sscanf(sizeStr, "%d", &size)

                hash, _ := f["md5Checksum"].(string)

                modTimeStr, _ := f["modifiedTime"].(string)
                modTime, _ := time.Parse(time.RFC3339, modTimeStr)

                files = append(files, CloudFile{
                    ID:           id,
                    Name:         name,
                    Path:         id, // Using ID as path
                    Hash:         hash,
                    Size:         size,
                    ModifiedTime: modTime,
                    Provider:     "gdrive",
                })
            }
        }

        if token, ok := result["nextPageToken"].(string); ok && token != "" {
            pageToken = token
        } else {
            break
        }
    }
    return files, nil
}

func (p *GDriveProvider) RenameFile(file *CloudFile, newName string) error {
    url := fmt.Sprintf("https://www.googleapis.com/drive/v3/files/%s", file.ID)
    body := map[string]interface{}{
        "name": newName,
    }
    jsonBody, _ := json.Marshal(body)

    req, _ := http.NewRequest("PATCH", url, bytes.NewBuffer(jsonBody))
    req.Header.Set("Authorization", "Bearer "+p.AccessToken)
    req.Header.Set("Content-Type", "application/json")

    resp, err := p.Client.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return fmt.Errorf("gdrive rename error: %s", resp.Status)
    }
    return nil
}

func (p *GDriveProvider) DeleteFile(file *CloudFile) error {
    url := fmt.Sprintf("https://www.googleapis.com/drive/v3/files/%s", file.ID)
    req, _ := http.NewRequest("DELETE", url, nil)
    req.Header.Set("Authorization", "Bearer "+p.AccessToken)

    resp, err := p.Client.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return fmt.Errorf("gdrive delete error: %s", resp.Status)
    }
    return nil
}
