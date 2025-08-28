# PR Comment Analyzer

A Rust CLI tool that analyzes GitHub pull request comments and calculates the average time spent per comment.

## Features

- Counts all types of comments (PR comments, review comments, issue comments) made by the token owner
- Supports multiple PR numbers for a single repository in a single run
- Handles GitHub API pagination automatically
- Calculates time per comment based on total time and comment count
- Allows adding additional comment count for comments that can't be easily detected
- **Auto-detects repository from git remote when run inside a git repository**
- **Supports both repository slug format (owner/repo) and full URLs**

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Run `cargo build --release`

## Usage

```bash
cargo run -- --token <your_github_token> --minutes <total_minutes> [--repository <repo>] [--additional <additional_comments>] <pr_number1> <pr_number2> ...
```

The repository can be specified in multiple ways:
- **Auto-detection** (default): If you're inside a git repository, it will automatically detect the GitHub repository from the remote origin
  - Supports both HTTPS (`https://github.com/owner/repo.git`) and SSH (`git@github.com:owner/repo.git`) remote formats
  - Automatically strips `.git` extensions
- **Repository slug**: `owner/repo` format (e.g., `Byron/vibed-gh-comment-check`)
- **Full URL**: `https://github.com/owner/repo` format

### Options

- `--token` or `-t`: GitHub personal access token (required)
- `--minutes` or `-m`: Total time spent in minutes (required)
- `--repository` or `-r`: GitHub repository (optional - auto-detects from git remote if not provided)
- `--additional` or `-a`: Additional comment count to add unconditionally to the total (optional, default: 0)

### Examples

**Auto-detection** (when run inside the target git repository):
```bash
cargo run -- --token ghp_abc123... --minutes 120 40 41 42
```

**Using repository slug format**:
```bash
cargo run -- --token ghp_abc123... --minutes 120 --repository owner/repo 40 41 42
```

**Using full URL format**:
```bash
cargo run -- --token ghp_abc123... --minutes 120 --repository https://github.com/owner/repo 40 41 42
```

**With additional comments for undetected comments**:
```bash
cargo run -- --token ghp_abc123... --minutes 120 --additional 15 --repository owner/repo 40 41 42
```

**Using short flags**:
```bash
cargo run -- -t ghp_abc123... -m 120 -a 15 -r owner/repo 40 41 42
```