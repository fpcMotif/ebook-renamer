package duplicates

import (
	"crypto/md5"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/ebook-renamer/go/internal/types"
)

// Allowed formats to keep
var allowedExtensions = map[string]bool{
	".pdf": true,
	".epub": true,
	".txt": true,
}

// DetectDuplicates finds duplicate files based on MD5 hash
func DetectDuplicates(files []types.FileInfo) ([][]string, []types.FileInfo, error) {
	// Filter to only allowed formats first
	var filteredFiles []types.FileInfo
	for _, file := range files {
		if allowedExtensions[file.Extension] {
			filteredFiles = append(filteredFiles, file)
		}
	}

	// Build hash map: file_hash -> list of file infos
	hashMap := make(map[string][]types.FileInfo)

	for _, fileInfo := range filteredFiles {
		if !fileInfo.IsFailedDownload && !fileInfo.IsTooSmall {
			hash, err := computeMD5(fileInfo.OriginalPath)
			if err != nil {
				// Skip files that can't be read
				continue
			}
			hashMap[hash] = append(hashMap[hash], fileInfo)
		}
	}

	// Group duplicates by hash and apply retention strategy
	var duplicateGroups [][]string
	duplicatePaths := make(map[string]bool)

	for _, fileInfos := range hashMap {
		if len(fileInfos) > 1 {
			// Multiple files with same hash - apply retention strategy
			keptFile := selectFileToKeep(fileInfos)
			
			var groupPaths []string
			groupPaths = append(groupPaths, keptFile.OriginalPath)
			
			for _, fileInfo := range fileInfos {
				if fileInfo.OriginalPath != keptFile.OriginalPath {
					duplicatePaths[fileInfo.OriginalPath] = true
					groupPaths = append(groupPaths, fileInfo.OriginalPath)
				}
			}

			duplicateGroups = append(duplicateGroups, groupPaths)
		}
	}

	// Return only non-duplicate files (including filtered out formats)
	var cleanFiles []types.FileInfo
	for _, file := range filteredFiles {
		if !duplicatePaths[file.OriginalPath] {
			cleanFiles = append(cleanFiles, file)
		}
	}

	return duplicateGroups, cleanFiles, nil
}

// selectFileToKeep selects the file to keep based on priority: normalized > shortest path > newest
func selectFileToKeep(files []types.FileInfo) types.FileInfo {
	// Priority 1: Already normalized files (have new_name set)
	var normalizedFiles []types.FileInfo
	var allFiles []types.FileInfo
	
	for _, file := range files {
		allFiles = append(allFiles, file)
		if file.NewName != nil {
			normalizedFiles = append(normalizedFiles, file)
		}
	}

	// Use normalized files if available, otherwise use all files
	candidates := normalizedFiles
	if len(candidates) == 0 {
		candidates = allFiles
	}

	// Priority 2: Shortest path (fewest directory components) among candidates
	type depthFile struct {
		depth int
		file  types.FileInfo
	}
	
	var candidatesWithDepth []depthFile
	minDepth := int(^uint(0) >> 1) // Max int
	
	for _, file := range candidates {
		depth := strings.Count(file.OriginalPath, string(filepath.Separator))
		candidatesWithDepth = append(candidatesWithDepth, depthFile{depth, file})
		if depth < minDepth {
			minDepth = depth
		}
	}
	
	// Filter to shallowest candidates
	var shallowestCandidates []types.FileInfo
	for _, df := range candidatesWithDepth {
		if df.depth == minDepth {
			shallowestCandidates = append(shallowestCandidates, df.file)
		}
	}

	// Priority 3: Newest modification time among the shallowest candidates
	if len(shallowestCandidates) == 0 {
		// Fallback: return first file
		return files[0]
	}

	newestFile := shallowestCandidates[0]
	for _, file := range shallowestCandidates[1:] {
		if file.ModifiedTime.After(newestFile.ModifiedTime) {
			newestFile = file
		}
	}

	return newestFile
}

// computeMD5 calculates MD5 hash of a file
func computeMD5(filePath string) (string, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	hash := md5.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	return fmt.Sprintf("%x", hash.Sum(nil)), nil
}

// DetectNameVariants groups files by normalized name (treating (1), (2), etc. as variants)
func DetectNameVariants(files []types.FileInfo) [][]int {
	// Group files by normalized name (treating (1), (2), etc. as variants)
	nameGroups := make(map[string][]int)

	for idx, fileInfo := range files {
		if fileInfo.NewName != nil {
			// Strip off (1), (2), etc. to find base name
			baseName := stripVariantSuffix(*fileInfo.NewName)
			nameGroups[baseName] = append(nameGroups[baseName], idx)
		}
	}

	// Keep only groups with duplicates
	var variants [][]int
	for _, group := range nameGroups {
		if len(group) > 1 {
			variants = append(variants, group)
		}
	}

	return variants
}

// stripVariantSuffix strips patterns like " (1)", " (2)", etc. from the end before extension
func stripVariantSuffix(filename string) string {
	// Match patterns like " (1)", " (2)", etc. at the end before extension
	if dotIdx := strings.LastIndex(filename, "."); dotIdx != -1 {
		namePart := filename[:dotIdx]
		extPart := filename[dotIdx:]
		
		// Remove variant suffix from name part
		if strings.HasSuffix(namePart, ")") {
			// Check if it matches pattern " (n)"
			openParen := strings.LastIndex(namePart, " (")
			if openParen != -1 {
				suffix := namePart[openParen:]
				if len(suffix) >= 4 && suffix[0] == ' ' && suffix[1] == '(' {
					// Check if content between parens is numeric
					content := suffix[2 : len(suffix)-1]
					isNumeric := true
					for _, r := range content {
						if r < '0' || r > '9' {
							isNumeric = false
							break
						}
					}
					if isNumeric {
						namePart = namePart[:openParen]
					}
				}
			}
		}
		
		return namePart + extPart
	} else {
		// No extension, just check for variant suffix
		if strings.HasSuffix(filename, ")") {
			openParen := strings.LastIndex(filename, " (")
			if openParen != -1 {
				suffix := filename[openParen:]
				if len(suffix) >= 4 && suffix[0] == ' ' && suffix[1] == '(' {
					content := suffix[2 : len(suffix)-1]
					isNumeric := true
					for _, r := range content {
						if r < '0' || r > '9' {
							isNumeric = false
							break
						}
					}
					if isNumeric {
						return filename[:openParen]
					}
				}
			}
		}
	}

	return filename
}
