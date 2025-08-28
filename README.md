# PR Comment Analyzer

A Rust CLI tool that analyzes GitHub pull request comments and calculates the average time spent per comment.

## Features

- Counts all types of comments (PR comments, review comments, issue comments) made by the token owner
- Supports multiple PR URLs in a single run
- Handles GitHub API pagination automatically
- Calculates time per comment based on total time and comment count

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Run `cargo build --release`

## Usage

```bash
cargo run -- --token <your_github_token> --minutes <total_minutes> --urls <pr_url1>,<pr_url2>,...