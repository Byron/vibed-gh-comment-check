# PR Comment Analyzer

A Rust CLI tool that analyzes GitHub pull request comments and calculates the average time spent per comment.

## Features

- Counts all types of comments (PR comments, review comments, issue comments) made by the token owner
- Supports multiple PR numbers for a single repository in a single run
- Handles GitHub API pagination automatically
- Calculates time per comment based on total time and comment count
- Allows adding additional comment count for comments that can't be easily detected

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Run `cargo build --release`

## Usage

```bash
cargo run -- --token <your_github_token> --minutes <total_minutes> [--additional <additional_comments>] <repository_url> <pr_number1> <pr_number2> ...
```

### Options

- `--token` or `-t`: GitHub personal access token (required)
- `--minutes` or `-m`: Total time spent in minutes (required)
- `--additional` or `-a`: Additional comment count to add unconditionally to the total (optional, default: 0)

### Examples

Basic usage:
```bash
cargo run -- --token ghp_abc123... --minutes 120 https://github.com/owner/repo 40 41 42
```

With additional comments for undetected comments:
```bash
cargo run -- --token ghp_abc123... --minutes 120 --additional 15 https://github.com/owner/repo 40 41 42
```

Using short flags:
```bash
cargo run -- -t ghp_abc123... -m 120 -a 15 https://github.com/owner/repo 40 41 42
```