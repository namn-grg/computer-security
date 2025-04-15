// group_8.rs
// This script is for educational purposes only within a controlled environment.

use eyre::Result;
use rand::{Rng, rng};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use reqwest::{Client, Response};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tracing::{debug, error, info};

/// Generates a random string of a given length.
fn random_string(len: usize) -> String {
    let mut rng = rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..62);
            match idx {
                0..=9 => (b'0' + idx as u8) as char,
                10..=35 => (b'a' + (idx - 10) as u8) as char,
                _ => (b'A' + (idx - 36) as u8) as char,
            }
        })
        .collect()
}

/// Generates a random hex string of a given length (for public key).
fn random_hex_string(len: usize) -> String {
    let mut rng = rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..16);
            match idx {
                0..=9 => (b'0' + idx as u8) as char,
                _ => (b'a' + (idx - 10) as u8) as char,
            }
        })
        .collect()
}

/// Generates a random password that meets requirements:
/// - at least 8 characters
/// - includes uppercase, lowercase, number, and special character
fn generate_password() -> String {
    let mut rng = rng();
    let uppercase = ('A'..='Z').collect::<Vec<char>>()[rng.random_range(0..26)];
    let lowercase = ('a'..='z').collect::<Vec<char>>()[rng.random_range(0..26)];
    let number = ('0'..='9').collect::<Vec<char>>()[rng.random_range(0..10)];
    let special_chars = "!@#$%^&*()_+".chars().collect::<Vec<char>>();
    let special = special_chars[rng.random_range(0..special_chars.len())];

    let mut password = vec![uppercase, lowercase, number, special];
    let remaining_length = 8 - password.len();

    // Add random additional characters
    for _ in 0..remaining_length {
        let additional_char = match rng.random_range(0..3) {
            0 => ('A'..='Z').collect::<Vec<char>>()[rng.random_range(0..26)],
            1 => ('a'..='z').collect::<Vec<char>>()[rng.random_range(0..26)],
            _ => ('0'..='9').collect::<Vec<char>>()[rng.random_range(0..10)],
        };
        password.push(additional_char);
    }

    // Shuffle the password characters
    for i in 0..password.len() {
        let j = rng.random_range(0..password.len());
        password.swap(i, j);
    }

    password.iter().collect()
}

/// Performs a GET request to load a page.
async fn get_page(client: &Client, url: &str) -> Result<Response> {
    let resp = client
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .send()
        .await?;
    Ok(resp)
}

/// Submits the signup form.
async fn submit_signup_form(
    client: &Client,
    signup_url: &str,
    target_url: &str,
    account_data: serde_json::Value,
) -> Result<Response> {
    // Set up headers to match the intercepted request
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"));
    headers.insert(REFERER, HeaderValue::from_str(signup_url)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "Accept",
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        "Sec-Ch-Ua",
        HeaderValue::from_static("\"Not:A-Brand\";v=\"24\", \"Chromium\";v=\"134\""),
    );
    headers.insert("Sec-Ch-Ua-Mobile", HeaderValue::from_static("?0"));
    headers.insert("Sec-Ch-Ua-Platform", HeaderValue::from_static("\"macOS\""));
    headers.insert("Origin", HeaderValue::from_str(signup_url)?);

    let resp = client
        .post(target_url)
        .headers(headers)
        .json(&account_data)
        .send()
        .await?;

    Ok(resp)
}

/// Creates a single account and returns the account info if successful
async fn create_account(
    task_id: usize,
    client: &Client,
    signup_url: &str,
    target_url: &str,
    account_num: usize,
) -> Result<Option<serde_json::Value>> {
    // Generate random data for the account
    let username = format!("user_{}", random_string(8));
    let email = format!("{}@example.com", random_string(10));
    let password = generate_password();
    let public_key = random_hex_string(64);

    let account_data = json!({
        "username": username,
        "email": email,
        "password": password,
        "public_key": public_key
    });

    info!(
        "Task #{}: Creating account #{}: username={}, email={}",
        task_id, account_num, username, email
    );

    let response = submit_signup_form(client, signup_url, target_url, account_data.clone()).await?;

    debug!(
        "Task #{}: Signup POST returned status: {}",
        task_id,
        response.status()
    );

    // Check for success - the intercepted response showed 201 Created
    if response.status().is_success() {
        info!(
            "Task #{}: Successfully submitted signup form for account #{}",
            task_id, account_num
        );

        Ok(Some(account_data))
    } else {
        error!(
            "Task #{}: Failed to submit signup for account #{}: Status {}",
            task_id,
            account_num,
            response.status()
        );
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let accounts_file = Arc::new(Mutex::new(
        File::create("group8_accounts.txt").expect("Failed to create accounts file"),
    ));

    let num_accounts = 100; // Adjust this number as needed
    let num_tasks = 20; // Number of concurrent tasks
    let accounts_per_task = num_accounts / num_tasks;
    let remainder = num_accounts % num_tasks;
    let successful_accounts = Arc::new(Mutex::new(0));

    info!(
        "Starting to create {} accounts with {} concurrent tasks",
        num_accounts, num_tasks
    );

    let signup_url = "https://192.168.2.240/"; // URL of the signup page
    let form_submit_url = "https://192.168.2.240/api/register/"; // Endpoint that processes the form

    let mut tasks = vec![];

    for task_id in 0..num_tasks {
        let task_accounts = if task_id < remainder {
            accounts_per_task + 1
        } else {
            accounts_per_task
        };

        let task_signup_url = signup_url.to_string();
        let task_submit_url = form_submit_url.to_string();
        let task_accounts_file = Arc::clone(&accounts_file);
        let task_successful_accounts = Arc::clone(&successful_accounts);

        let task = task::spawn(async move {
            let client = Client::builder()
                .danger_accept_invalid_certs(true)
                .cookie_store(true)
                .build()
                .expect("Failed to build HTTP client");

            match get_page(&client, &task_signup_url).await {
                Ok(page) => {
                    if !page.status().is_success() {
                        error!(
                            "Task #{}: Failed to load signup page. Status: {}",
                            task_id,
                            page.status()
                        );
                    }
                }
                Err(e) => {
                    error!("Task #{}: Failed to connect to signup page: {}", task_id, e);
                }
            }

            // Create the assigned number of accounts
            for i in 0..task_accounts {
                let account_num = task_id * accounts_per_task + i + 1;

                match create_account(
                    task_id,
                    &client,
                    &task_signup_url,
                    &task_submit_url,
                    account_num,
                )
                .await
                {
                    Ok(Some(account_data)) => {
                        let mut file = task_accounts_file.lock().await;
                        if let Err(e) = writeln!(
                            file,
                            "{}",
                            serde_json::to_string_pretty(&account_data).unwrap_or_default()
                        ) {
                            error!("Task #{}: Failed to write account info: {}", task_id, e);
                        }

                        let mut count = task_successful_accounts.lock().await;
                        *count += 1;
                    }
                    Ok(None) => {
                        info!(
                            "Task #{}: Account #{} failed to complete",
                            task_id, account_num
                        );
                    }
                    Err(e) => {
                        error!(
                            "Task #{}: Error creating account #{}: {}",
                            task_id, account_num, e
                        );
                    }
                }

                // sleep(Duration::from_millis(100)).await;
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        task.await?;
    }

    let successful = *successful_accounts.lock().await;
    info!(
        "Finished creating accounts. Successfully created {}/{} accounts.",
        successful, num_accounts
    );
    info!("Account information has been saved to group8_accounts.txt");
    Ok(())
}
