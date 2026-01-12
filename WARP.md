# Shell Command Preferences

## Use Efficient Rust-Based Tools
Always prefer modern Rust-based alternatives over traditional Unix tools for better performance:

- **fd** instead of `find` - faster file finding
- **rg** (ripgrep) instead of `grep` - faster text search (already in rules)
- **bat** instead of `cat` - syntax highlighting and paging
- **exa** instead of `ls` - better file listing
- **dust** instead of `du` - disk usage visualization
- **tokei** instead of `cloc` - code statistics
- **sd** instead of `sed` - simpler find/replace
- **procs** instead of `ps` - modern process viewer
- **hyperfine** instead of `time` - benchmarking
- **delta** instead of `diff` - better diffs

## Advanced fzf Usage
Combine fzf with Rust tools for powerful interactive workflows:
- Use `fd` for file input to fzf
- Use `rg` for content search with fzf
- Use `bat` for preview windows
- Leverage fzf bindings: `--multi`, `--preview`, `--bind` for custom actions
