use std::format;
use std::fs::File;
use std::{println, process::exit};

use std::cmp::min;
use std::io::Write;

use reqwest::Client;
use reqwest::Response;
use scraper::{Html, Selector};

use log::error;
use log::info;
use log::LevelFilter;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};

use dialoguer::{theme::ColorfulTheme, Select};

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    let client = Client::new();

    let mut tuxfamily_link = "https://downloads.tuxfamily.org/godotengine".to_string();

    info!("Fetching '{}' ", &tuxfamily_link);

    let res = client
        .get(&tuxfamily_link)
        .send()
        .await
        .unwrap_or_else(|err| {
            error!("fetching godot's tuxfamily link: {}", err);
            exit(1);
        });

    let filters = vec![
        "Parent Directory".to_string(),
        "toolchains".to_string(),
        "patreon".to_string(),
        "media".to_string(),
        "testing".to_string(),
    ];
    let mut versions = get_choices_from_html_tags(res, filters).await;

    // Show Latest Version First.
    versions.reverse();

    // loop and keep fetching subversions until we reach the endpoint file tar.xz or zip.. etc
    loop {
        let selected_version = choose(&mut versions);

        println!("You selected version: {}", selected_version);

        tuxfamily_link = format!("{}/{}", tuxfamily_link, selected_version).to_string();

        if tuxfamily_link.contains(".tar.xz")
            || tuxfamily_link.contains(".zip")
            || tuxfamily_link.contains(".aar")
            || tuxfamily_link.contains(".aab")
            || tuxfamily_link.contains(".apk")
            || tuxfamily_link.contains(".tpz")
        {
            break;
        }

        // Fetch subversions
        info!("Fetching '{}' ", &tuxfamily_link);
        let resp = client
            .get(&tuxfamily_link)
            .send()
            .await
            .unwrap_or_else(|err| {
                error!("fetching godot's subversion tuxfamily link: {}", err);
                exit(1);
            });

        let filters = vec!["Parent Directory".to_string()];
        versions = get_choices_from_html_tags(resp, filters).await;

        // Show Latest Sub Version First.
        versions.reverse();
    }

    info!("Fetching File: '{}' ", &tuxfamily_link);
    let _ = download_file(&tuxfamily_link).await;
}

async fn get_choices_from_html_tags(resp: Response, filters: Vec<String>) -> Vec<String> {
    let body = match resp.text().await {
        Ok(text_body) => text_body,
        Err(err) => {
            error!("{}", err);
            exit(1);
        }
    };

    let fragment = Html::parse_document(&body);

    let selector = match Selector::parse("a") {
        Ok(a_tag_selector) => a_tag_selector,
        Err(err) => {
            error!("Geting a tag: {}", err);
            exit(1);
        }
    };

    // Regex for x.y.z
    let mut versions = Vec::new();

    for element in fragment.select(&selector) {
        let text = element.text().collect::<Vec<_>>().join("");
        if filters.iter().all(|f| !text.contains(f)) {
            versions.push(text.trim_end_matches('/').to_string());
        }
    }

    versions
}

pub async fn download_file(download_url: &str) -> Result<(), String> {
    // get filename with extension from url
    let path = download_url.split('/').last().unwrap_or_else(|| {
        error!("Failed to get filename from '{}'", &download_url);
        exit(-1);
    });

    let client = Client::new();
    // Reqwest setup
    let res = client
        .get(download_url.clone())
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &download_url)))?;
    let total_size = res.content_length().ok_or(format!(
        "Failed to get content length from '{}'",
        &download_url
    ))?;

    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", download_url));

    // download chunks
    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", download_url, path));
    Ok(())
}

fn choose(choices: &mut [String]) -> String {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a version")
        .default(0)
        .items(choices)
        .interact()
        .unwrap();

    choices[selection].clone()
}
