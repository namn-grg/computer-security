// main.rs
// This script is for educational use only in a controlled and authorized testing environment.

use rand::distr::Alphanumeric;
use rand::Rng;
use reqwest::blocking::Client;
use reqwest::header::{USER_AGENT, CONTENT_TYPE};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{self, fmt, EnvFilter};

use std::error::Error;
use std::thread;
use std::time::Duration;

/// Generates a random string of a given length.
fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing with a format that includes timestamps
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    
        let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    // The registration page URL which also initializes your session.
    let register_url = "https://192.168.3.8/register.php";
    
    // Perform a GET to the signup page to establish session cookies and any tokens.
    let initial_response = client.get(register_url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .send()?;
        
    if !initial_response.status().is_success() {
        eprintln!("Failed to load the registration page. Status: {}", initial_response.status());
        return Ok(());
    }

    // Number of accounts to create.
    let num_accounts = 10;
    
    // The POST target: action URL as defined in the form.
    let target_url = "https://192.168.3.8/execute_file.php?filename=process_register_form.php";

    info!("Starting account creation process. Target: {}", num_accounts);
    for i in 0..num_accounts {
        // Generate random data for each field.
        let random_part = random_string(8);
        let email = format!("{}@example.com", random_string(10));
        let full_name = format!("user_{}", random_part);
        let username = format!("user_{}", random_part);
        let password = format!("Passw0rd!{}", random_part);
        // Generate a fake phone number (e.g., "555-1234").
        let phone_number = format!("555555{}", rand::rng().random_range(1000..10000));

        // Log the account details being created
        info!("Creating account #{}: username={}, email={}, phone={}, password={}", i + 1, username, email, phone_number, password);

        // Build the POST form parameters matching the registration form's input names.
        let params = [
            ("email", email.as_str()),
            ("phone_number", phone_number.as_str()),
            ("full_name", full_name.as_str()),
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];

        // Send the POST request.
        let response = client.post(target_url)
            .header(USER_AGENT, register_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&params)
            .send()?;

        // Log response status
        info!("Account #{} created. Status: {}", i + 1, response.status());

        // Optional: add a short delay between requests to avoid overwhelming the server.
        thread::sleep(Duration::from_millis(500));
    }

    info!("Finished creating {} accounts.", num_accounts);
    Ok(())
}