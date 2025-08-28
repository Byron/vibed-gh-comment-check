# PR Comment Analyzer

A Rust CLI tool that analyzes GitHub pull request comments and calculates the average time spent per comment.

## Features

- Counts all types of comments (PR comments, review comments, issue comments) made by the token owner
- Supports multiple PR numbers for a single repository in a single run
- Handles GitHub API pagination automatically
- Calculates time per comment based on total time and comment count

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Run `cargo build --release`

## Usage

```bash
cargo run -- --token <your_github_token> --minutes <total_minutes> <repository_url> <pr_number1> <pr_number2> ...
```

For example:
```bash
cargo run -- --token ghp_abc123... --minutes 120 https://github.com/owner/repo 40 41 42
```