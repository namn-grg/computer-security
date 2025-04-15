// main.rs
// This script is for educational use only in a controlled and authorized testing environment.

use eyre::Result;
use rand::Rng;
use rand::distr::Alphanumeric;
use reqwest::header::{CONTENT_TYPE, REFERER, USER_AGENT};
use reqwest::multipart::{Form, Part};
use reqwest::{Client, Response};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

/// Generates a random string of a given length.
fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Performs a GET request and returns the response.
async fn get_page(client: &Client, url: &str) -> Result<Response> {
    let resp = client
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .send()
        .await?;
    Ok(resp)
}

/// Submits a URL-encoded registration form to create an account.
async fn submit_registration_form(
    client: &Client,
    register_url: &str,
    target_url: &str,
    email: &str,
    phone_number: &str,
    full_name: &str,
    username: &str,
    password: &str,
) -> Result<Response> {
    let params = [
        ("email", email),
        ("phone_number", phone_number),
        ("full_name", full_name),
        ("username", username),
        ("password", password),
    ];

    let resp = client
        .post(target_url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .header(REFERER, register_url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    Ok(resp)
}

/// Submits the profile setup form with a file upload.
async fn submit_profile_setup(
    client: &Client,
    profile_setup_url: &str,
    target_profile_url: &str,
    user_display_name: &str,
    bio: &str,
    image_bytes: Vec<u8>,
    image_filename: &str,
) -> Result<Response> {
    // Create a file part for the profile image.
    let file_part = Part::bytes(image_bytes)
        .file_name(image_filename.to_string())
        .mime_str("image/png")?; // Adjust mime type if needed

    // Build a multipart form with the file and text fields.
    let form = Form::new()
        .part("profile_picture_picker", file_part)
        .text("user_display_name", user_display_name.to_string())
        .text("bio", bio.to_string());

    let resp = client
        .post(target_profile_url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .header(REFERER, profile_setup_url)
        .multipart(form)
        .send()
        .await?;
    Ok(resp)
}

/// Creates a single account and returns the account info if successful
async fn create_account(
    task_id: usize,
    client: &Client,
    register_url: &str,
    image_bytes: Vec<u8>,
) -> Result<Option<String>> {
    // Generate random data for the account
    let random_part = random_string(8);
    let email = format!("{}@example.com", random_string(10));
    let full_name = format!("user_{}", random_part);
    let username = format!("user_{}", random_part);
    let password = format!("Passw0rd!{}", random_part);
    let phone_number = format!("555555{}", rand::rng().random_range(1000..10000));

    info!(
        "Task #{}: Creating account: username={}, email={}, phone={}, password={}",
        task_id, username, email, phone_number, password
    );

    let reg_target_url = "https://192.168.3.8/execute_file.php?filename=process_register_form.php";
    let reg_response = submit_registration_form(
        client,
        register_url,
        reg_target_url,
        &email,
        &phone_number,
        &full_name,
        &username,
        &password,
    )
    .await?;

    debug!(
        "Task #{}: Registration POST returned status: {}",
        task_id,
        reg_response.status()
    );

    sleep(Duration::from_millis(500)).await;

    let profile_setup_url = "https://192.168.3.8/profile_setup.php";
    info!("Task #{}: Fetching profile setup page", task_id);
    let profile_page = get_page(client, profile_setup_url).await?;
    if !profile_page.status().is_success() {
        error!(
            "Task #{}: Failed to load profile setup page. Status: {}",
            task_id,
            profile_page.status()
        );
        return Ok(None);
    }

    let image_filename = "profile.png";
    let user_display_name = username.clone();
    let bio = format!("Hi I'm {}!", username);
    let profile_target_url =
        "https://192.168.3.8/execute_file.php?filename=process_profile_setup.php";

    info!(
        "Task #{}: Submitting profile setup with image upload",
        task_id
    );
    let profile_response = submit_profile_setup(
        client,
        profile_setup_url,
        profile_target_url,
        &user_display_name,
        &bio,
        image_bytes,
        image_filename,
    )
    .await?;

    debug!(
        "Task #{}: Profile setup POST returned status: {}",
        task_id,
        profile_response.status()
    );

    // Check if the account was created successfully
    if profile_response.url().path() == "/index.php" {
        info!(
            "Task #{}: Registration flow complete. Account {} registered and profile setup done.",
            task_id, username
        );

        // Return the account information as a formatted string
        let account_info = format!(
            "Username: {}, Email: {}, Phone: {}, Password: {}, FullName: {}",
            username, email, phone_number, password, full_name
        );

        Ok(Some(account_info))
    } else {
        info!(
            "Task #{}: Registration flow complete, but final redirection did not land on index.php. Final URL: {}",
            task_id,
            profile_response.url()
        );
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create a file to store account information
    let accounts_file = Arc::new(Mutex::new(
        File::create("accounts.txt").expect("Failed to create accounts file"),
    ));

    let num_accounts = 500;
    let num_tasks = 10;
    let image_bytes = fs::read("profile.png")?;
    let accounts_per_task = num_accounts / num_tasks;
    let remainder = num_accounts % num_tasks;
    let successful_accounts = Arc::new(Mutex::new(0));

    info!(
        "Starting to create {} accounts with {} concurrent tasks",
        num_accounts, num_tasks
    );

    let mut tasks = vec![];

    for task_id in 0..num_tasks {
        let task_accounts = if task_id < remainder {
            accounts_per_task + 1
        } else {
            accounts_per_task
        };

        let register_url = "https://192.168.3.8/register.php".to_string();
        let task_image_bytes = image_bytes.clone();
        let task_accounts_file = Arc::clone(&accounts_file);
        let task_successful_accounts = Arc::clone(&successful_accounts);

        let task = task::spawn(async move {
            let client = Client::builder()
                .danger_accept_invalid_certs(true)
                .cookie_store(true)
                .build()
                .expect("Failed to build HTTP client");

            match get_page(&client, &register_url).await {
                Ok(reg_page) => {
                    if !reg_page.status().is_success() {
                        error!(
                            "Task #{}: Failed to load registration page. Status: {}",
                            task_id,
                            reg_page.status()
                        );
                        return;
                    }
                }
                Err(e) => {
                    error!(
                        "Task #{}: Failed to connect to registration page: {}",
                        task_id, e
                    );
                    return;
                }
            }

            // Create the assigned number of accounts
            for i in 0..task_accounts {
                match create_account(task_id, &client, &register_url, task_image_bytes.clone())
                    .await
                {
                    Ok(Some(account_info)) => {
                        // Write successful account info to the file
                        let mut file = task_accounts_file.lock().await;
                        if let Err(e) = writeln!(file, "{}", account_info) {
                            error!("Task #{}: Failed to write account info: {}", task_id, e);
                        }

                        // Increment our success counter
                        let mut count = task_successful_accounts.lock().await;
                        *count += 1;
                    }
                    Ok(None) => {
                        info!("Task #{}: Account #{} failed to complete", task_id, i + 1);
                    }
                    Err(e) => {
                        error!(
                            "Task #{}: Error creating account #{}: {}",
                            task_id,
                            i + 1,
                            e
                        );
                    }
                }

                sleep(Duration::from_millis(200)).await;
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
    info!("Account information has been saved to accounts_26.txt");
    Ok(())
}
