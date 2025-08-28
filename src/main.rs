use anyhow::{Context, Result};
use clap::{Arg, Command};
use reqwest::Client;
use serde_json::Value;
use std::process::{self, Command as ProcessCommand};

#[derive(Debug)]
struct PrCommentCounts {
    pr_number: u32,
    pr_comments: u32,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_app().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run_app() -> Result<()> {
    let matches = Command::new("pr-comment-analyzer")
        .version("1.0")
        .author("Your Name")
        .about("Analyzes GitHub PR comments and calculates time per comment")
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("TOKEN")
                .help("GitHub personal access token")
                .required(true),
        )
        .arg(
            Arg::new("minutes")
                .short('m')
                .long("minutes")
                .value_name("MINUTES")
                .help("Total time spent in minutes")
                .required(true)
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("repository")
                .short('r')
                .long("repository")
                .value_name("REPOSITORY")
                .help("GitHub repository (e.g., owner/repo or https://github.com/owner/repo). If not provided, auto-detects from git remote."),
        )
        .arg(
            Arg::new("additional")
                .short('a')
                .long("additional")
                .value_name("ADDITIONAL")
                .help("Additional comment count to add unconditionally to the total")
                .value_parser(clap::value_parser!(u32))
                .default_value("0"),
        )
        .arg(
            Arg::new("pr_numbers")
                .value_name("PR_NUMBERS")
                .help("PR numbers to analyze")
                .required(true)
                .num_args(1..)
                .index(1),
        )
        .get_matches();

    let token = matches.get_one::<String>("token").context("Token argument is required")?;
    let minutes = *matches.get_one::<u32>("minutes").context("Minutes argument is required")?;
    let additional = *matches.get_one::<u32>("additional").context("Additional argument should have default value")?;
    
    // Get repository - either from flag or auto-detect
    let repository = match matches.get_one::<String>("repository") {
        Some(repo) => repo.clone(),
        None => {
            match auto_detect_repository() {
                Ok(repo) => {
                    println!("Auto-detected repository: {}", repo);
                    repo
                },
                Err(e) => {
                    eprintln!("Error: Failed to auto-detect repository: {}", e);
                    eprintln!("Please specify the repository using -r/--repository flag.");
                    process::exit(1);
                }
            }
        }
    };
    
    let pr_numbers: Result<Vec<u32>> = matches
        .get_many::<String>("pr_numbers")
        .context("PR numbers are required")?
        .map(|s| s.parse::<u32>().context(format!("Invalid PR number: {}", s)))
        .collect();
    let pr_numbers = pr_numbers?;

    run(token, minutes, additional, &repository, pr_numbers).await
}

async fn process_single_pr(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u32,
    user_login: &str,
) -> Result<PrCommentCounts> {
    // Run all three comment fetching operations in parallel for this PR
    let pr_comments =
        get_pr_comments(client, token, owner, repo, pr_number).await?;
    let pr_comments = count_user_comments(&pr_comments, user_login);

    Ok(PrCommentCounts {
        pr_number,
        pr_comments,
    })
}

async fn run(token: &str, minutes: u32, additional: u32, repository: &str, pr_numbers: Vec<u32>) -> Result<()> {
    let client = Client::new();
    
    // First, get the authenticated user's login
    let user_login = get_authenticated_user(&client, token).await?;
    println!("Analyzing comments for user: {}", user_login);
    
    // Parse the repository URL to get owner and repo
    let (owner, repo) = parse_repository_url(repository)?;
    println!("Repository: {}/{}", owner, repo);
    
    // Create futures for processing all PRs in parallel
    let pr_futures: Vec<_> = pr_numbers
        .iter()
        .map(|&pr_number| {
            let client = &client;
            let token = token;
            let owner = &owner;
            let repo = &repo;
            let user_login = &user_login;
            async move {
                process_single_pr(client, token, owner, repo, pr_number, user_login).await
            }
        })
        .collect();
    
    // Run all PR processing in parallel
    let pr_results = futures::future::try_join_all(pr_futures).await?;
    
    let mut total_comments = 0;
    
    // Display results for each PR
    for result in &pr_results {
        println!("\nAnalyzing PR #{}: https://github.com/{}/{}/pull/{}", result.pr_number, owner, repo, result.pr_number);
        
        let pr_total = result.pr_comments ;
        total_comments += pr_total;
        
        println!("  PR comments: {}", result.pr_comments);
    }
    
    println!("\n=== SUMMARY ===");
    println!("Total comments across all PRs: {}", total_comments);
    if additional > 0 {
        println!("Additional comments: {}", additional);
        total_comments += additional;
        println!("Total comments (including additional): {}", total_comments);
    }
    println!("Total time: {} minutes", minutes);
    
    if total_comments > 0 {
        let minutes_per_comment = minutes as f64 / total_comments as f64;
        println!("Time per comment: {:.2} minutes", minutes_per_comment);
    } else {
        println!("No comments found for the authenticated user.");
    }
    
    Ok(())
}

async fn get_authenticated_user(client: &Client, token: &str) -> Result<String> {
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "pr-comment-analyzer")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to get user info: {}", response.status()));
    }
    
    let user: Value = response.json().await?;
    let login = user["login"]
        .as_str()
        .context("Unable to get user login from API response")?
        .to_string();
    
    Ok(login)
}

fn parse_repository_url(url: &str) -> Result<(String, String)> {
    // Check if it's a slug format (org/repo)
    if !url.contains('/') {
        return Err(anyhow::anyhow!("Invalid repository format. Expected: org/repo or https://github.com/org/repo"));
    }
    
    // If it doesn't contain protocol, treat as slug format
    if !url.starts_with("http") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository slug format. Expected: org/repo"));
        }
        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        return Ok((owner, repo));
    }
    
    // Handle full URL format: https://github.com/owner/repo
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    
    if parts.len() < 5 || parts[2] != "github.com" {
        return Err(anyhow::anyhow!("Invalid GitHub repository URL format. Expected: https://github.com/owner/repo"));
    }
    
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();
    
    Ok((owner, repo))
}

fn auto_detect_repository() -> Result<String> {
    // Try to get the remote URL of the current branch's HEAD
    let output = ProcessCommand::new("git")
        .args(&["config", "--get", "remote.origin.url"])
        .output()
        .context("Failed to run git command. Make sure git is installed and you're in a git repository.")?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get git remote URL. Make sure you're in a git repository with a remote origin."));
    }
    
    let remote_url = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim()
        .to_string();
    
    if remote_url.is_empty() {
        return Err(anyhow::anyhow!("No remote origin URL found in git repository."));
    }
    
    // Convert various git URL formats to GitHub repository format
    if remote_url.starts_with("git@github.com:") {
        // SSH format: git@github.com:owner/repo.git
        let repo_part = remote_url.strip_prefix("git@github.com:")
            .context("Failed to strip SSH prefix from git remote URL")?;
        let repo_part = repo_part.strip_suffix(".git").unwrap_or(repo_part);
        return Ok(repo_part.to_string());
    } else if remote_url.starts_with("https://github.com/") {
        // HTTPS format: https://github.com/owner/repo.git
        let repo_part = remote_url.strip_prefix("https://github.com/")
            .context("Failed to strip HTTPS prefix from git remote URL")?;
        let repo_part = repo_part.strip_suffix(".git").unwrap_or(repo_part);
        return Ok(repo_part.to_string());
    } else {
        return Err(anyhow::anyhow!("Unsupported git remote URL format: {}. Only GitHub repositories are supported.", remote_url));
    }
}

async fn get_pr_comments(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Value>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/comments",
        owner, repo, pr_number
    );
    
    get_paginated_comments(client, token, &url).await
}

async fn get_paginated_comments(
    client: &Client,
    token: &str,
    url: &str,
) -> Result<Vec<Value>> {
    let mut all_comments = Vec::new();
    let mut current_url = url.to_string();
    
    loop {
        let response = client
            .get(&current_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "pr-comment-analyzer")
            .query(&[("per_page", "100")])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API request failed: {}", response.status()));
        }
        
        // Check for next page in Link header
        let link_header = response.headers().get("link");
        let next_url = link_header
            .and_then(|h| h.to_str().ok())
            .and_then(|h| parse_next_link(h));
        
        let comments: Vec<Value> = response.json().await?;
        all_comments.extend(comments);
        
        match next_url {
            Some(url) => current_url = url,
            None => break,
        }
    }
    
    Ok(all_comments)
}

fn parse_next_link(link_header: &str) -> Option<String> {
    // Parse Link header to find "next" relation
    for link in link_header.split(',') {
        let parts: Vec<&str> = link.trim().split(';').collect();
        if parts.len() == 2 {
            let url = parts[0].trim_start_matches('<').trim_end_matches('>');
            let rel = parts[1].trim();
            if rel.contains("rel=\"next\"") {
                return Some(url.to_string());
            }
        }
    }
    None
}

fn count_user_comments(comments: &[Value], user_login: &str) -> u32 {
    comments
        .iter()
        .filter(|comment| {
            comment["user"]["login"]
                .as_str()
                .map_or(false, |login| login == user_login)
        })
        .count() as u32
}