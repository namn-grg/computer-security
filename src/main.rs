// main.rs
// This script is for educational use only in a controlled and authorized testing environment.

use rand::Rng;
use rand::distr::Alphanumeric;
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, REFERER, USER_AGENT};
use std::error::Error;
use std::fs;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, trace};

/// Generates a random string of a given length.
fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Performs a GET request and returns the response.
fn get_page(client: &Client, url: &str) -> Result<Response, Box<dyn Error>> {
    let resp = client
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (compatible; EthicalHacker/1.0)")
        .send()?;
    Ok(resp)
}

/// Submits a URL-encoded registration form to create an account.
fn submit_registration_form(
    client: &Client,
    register_url: &str,
    target_url: &str,
    email: &str,
    phone_number: &str,
    full_name: &str,
    username: &str,
    password: &str,
) -> Result<Response, Box<dyn Error>> {
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
        .send()?;

    Ok(resp)
}

/// Submits the profile setup form with a file upload.
fn submit_profile_setup(
    client: &Client,
    profile_setup_url: &str,
    target_profile_url: &str,
    user_display_name: &str,
    bio: &str,
    image_bytes: Vec<u8>,
    image_filename: &str,
) -> Result<Response, Box<dyn Error>> {
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
        .send()?;
    Ok(resp)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the tracing subscriber to output logs to stdout.
    tracing_subscriber::fmt::init();

    // Build a client that accepts invalid certificates (helpful in lab environments with self-signed certs).
    // We are using the default redirect policy here; note that your earlier logs indicated redirection.
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()?;

    let register_url = "https://192.168.3.8/register.php";
    info!("Fetching registration page at {}", register_url);
    let reg_page = get_page(&client, register_url)?;
    if !reg_page.status().is_success() {
        error!(
            "Failed to load registration page. Status: {}",
            reg_page.status()
        );
        return Ok(());
    }
    info!(
        "Registration page loaded with status: {}",
        reg_page.status()
    );

    // Set how many accounts you want to create
    let num_accounts = 500;

    for i in 0..num_accounts {
        // Generate random data for each account.
        let random_part = random_string(8);
        let email = format!("{}@example.com", random_string(10));
        let full_name = format!("user_{}", random_part);
        let username = format!("user_{}", random_part);
        let password = format!("Passw0rd!{}", random_part);
        let phone_number = format!("555555{}", rand::thread_rng().gen_range(1000..10000));

        info!(
            "Creating account #{}: username={}, email={}, phone={}, password={}",
            i + 1,
            username,
            email,
            phone_number,
            password
        );

        let reg_target_url =
            "https://192.168.3.8/execute_file.php?filename=process_register_form.php";
        let reg_response = submit_registration_form(
            &client,
            register_url,
            reg_target_url,
            &email,
            &phone_number,
            &full_name,
            &username,
            &password,
        )?;
        debug!(
            "Registration POST returned status: {}",
            reg_response.status()
        );
        debug!("Registration response final URL: {}", reg_response.url());

        thread::sleep(Duration::from_millis(500));

        let profile_setup_url = "https://192.168.3.8/profile_setup.php";
        info!("Fetching profile setup page at {}", profile_setup_url);
        let profile_page = get_page(&client, profile_setup_url)?;
        if !profile_page.status().is_success() {
            error!(
                "Failed to load profile setup page. Status: {}",
                profile_page.status()
            );
            return Ok(());
        }
        info!(
            "Profile setup page loaded, status: {}",
            profile_page.status()
        );

        let image_filename = "profile.png";
        let image_bytes = fs::read(image_filename)?;

        let user_display_name = username.clone();
        let bio = format!("Hi I'm {}!", username);
        let profile_target_url =
            "https://192.168.3.8/execute_file.php?filename=process_profile_setup.php";
        info!("Submitting profile setup with image upload.");
        let profile_response = submit_profile_setup(
            &client,
            profile_setup_url,
            profile_target_url,
            &user_display_name,
            &bio,
            image_bytes,
            image_filename,
        )?;
        debug!(
            "Profile setup POST returned status: {}",
            profile_response.status()
        );
        trace!("Profile setup response URL: {}", profile_response.url());

        // Check if the final destination matches the expected completion page.
        if profile_response.url().path() == "/index.php" {
            info!(
                "Registration flow complete. Account {} registered and profile setup done.",
                username
            );
        } else {
            info!(
                "Registration flow complete, but final redirection did not land on index.php. Final URL: {}",
                profile_response.url()
            );
        }
    }

    info!("Finished creating {} accounts.", num_accounts);
    Ok(())
}
