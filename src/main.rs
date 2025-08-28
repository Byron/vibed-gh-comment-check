use clap::{Arg, Command};
use reqwest::Client;
use serde_json::Value;
use std::error::Error;
use std::process;

#[tokio::main]
async fn main() {
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
                .value_name("REPOSITORY")
                .help("GitHub repository URL (e.g., https://github.com/owner/repo)")
                .required(true)
                .index(1),
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
                .index(2),
        )
        .get_matches();

    let token = matches.get_one::<String>("token").unwrap();
    let minutes = *matches.get_one::<u32>("minutes").unwrap();
    let additional = *matches.get_one::<u32>("additional").unwrap();
    let repository = matches.get_one::<String>("repository").unwrap();
    let pr_numbers: Vec<u32> = matches
        .get_many::<String>("pr_numbers")
        .unwrap()
        .map(|s| s.parse().expect("Invalid PR number"))
        .collect();

    if let Err(e) = run(token, minutes, additional, repository, pr_numbers).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run(token: &str, minutes: u32, additional: u32, repository: &str, pr_numbers: Vec<u32>) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    
    // First, get the authenticated user's login
    let user_login = get_authenticated_user(&client, token).await?;
    println!("Analyzing comments for user: {}", user_login);
    
    // Parse the repository URL to get owner and repo
    let (owner, repo) = parse_repository_url(repository)?;
    println!("Repository: {}/{}", owner, repo);
    
    let mut total_comments = 0;
    
    for pr_number in pr_numbers {
        println!("\nAnalyzing PR #{}: https://github.com/{}/{}/pull/{}", pr_number, owner, repo, pr_number);
        
        // Get PR comments
        let pr_comments = get_pr_comments(&client, token, &owner, &repo, pr_number).await?;
        let pr_comment_count = count_user_comments(&pr_comments, &user_login);
        
        // Get review comments
        let review_comments = get_review_comments(&client, token, &owner, &repo, pr_number).await?;
        let review_comment_count = count_user_comments(&review_comments, &user_login);
        
        // Get issue comments (PRs are issues in GitHub API)
        let issue_comments = get_issue_comments(&client, token, &owner, &repo, pr_number).await?;
        let issue_comment_count = count_user_comments(&issue_comments, &user_login);
        
        let pr_total = pr_comment_count + review_comment_count + issue_comment_count;
        total_comments += pr_total;
        
        println!("  PR comments: {}", pr_comment_count);
        println!("  Review comments: {}", review_comment_count);
        println!("  Issue comments: {}", issue_comment_count);
        println!("  Total for this PR: {}", pr_total);
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

async fn get_authenticated_user(client: &Client, token: &str) -> Result<String, Box<dyn Error>> {
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "pr-comment-analyzer")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to get user info: {}", response.status()).into());
    }
    
    let user: Value = response.json().await?;
    let login = user["login"]
        .as_str()
        .ok_or("Unable to get user login")?
        .to_string();
    
    Ok(login)
}

fn parse_repository_url(url: &str) -> Result<(String, String), Box<dyn Error>> {
    // Expected format: https://github.com/owner/repo
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    
    if parts.len() < 5 || parts[2] != "github.com" {
        return Err("Invalid GitHub repository URL format. Expected: https://github.com/owner/repo".into());
    }
    
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();
    
    Ok((owner, repo))
}

async fn get_pr_comments(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/comments",
        owner, repo, pr_number
    );
    
    get_paginated_comments(client, token, &url).await
}

async fn get_review_comments(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
        owner, repo, pr_number
    );
    
    get_paginated_comments(client, token, &url).await
}

async fn get_issue_comments(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues/{}/comments",
        owner, repo, pr_number
    );
    
    get_paginated_comments(client, token, &url).await
}

async fn get_paginated_comments(
    client: &Client,
    token: &str,
    url: &str,
) -> Result<Vec<Value>, Box<dyn Error>> {
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
            return Err(format!("API request failed: {}", response.status()).into());
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