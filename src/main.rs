use std::format;
use std::{io::stdin, println, process::exit};

use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::Client as NonBlockingClient;
use reqwest::Response as NonBlockingResponse;
use scraper::{Html, Selector};

use log::error;
use log::info;
use log::LevelFilter;

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    let client = Client::new();

    let tuxfamily_link = "https://downloads.tuxfamily.org/godotengine";

    info!("Fetching '{}' ", &tuxfamily_link);

    let res = client.get(tuxfamily_link).send().unwrap_or_else(|err| {
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
    let mut versions = get_choices_from_html_tags(res, filters);

    // Show Latest Version First.
    versions.reverse();

    let selected_version = choose(&versions);

    println!("You selected version: {}", selected_version);

    let subversion_url = format!("{}/{}/", tuxfamily_link, selected_version);

    // Fetch subversions
    let resp = client.get(subversion_url).send().unwrap_or_else(|err| {
        error!("fetching godot's subversion tuxfamily link: {}", err);
        exit(1);
    });

    let filters = vec!["Parent Directory".to_string()];
    let mut subversions = get_choices_from_html_tags(resp, filters);

    // Show Latest Sub Version First.
    subversions.reverse();
    let selected_subversion = choose(&subversions);

    let platform_url = format!(
        "{}/{}/{}/",
        tuxfamily_link, selected_version, selected_subversion
    );
    let resp = client.get(platform_url).send().unwrap_or_else(|err| {
        error!("fetching godot's tuxfamily link: {}", err);
        exit(1);
    });

    let filters = vec!["Parent Directory".to_string()];
    let platforms = get_choices_from_html_tags(resp, filters);
    let selected_platform = choose(&platforms);
    let _ = download_and_extract(format!(
        "{}/{}/{}/{}/",
        tuxfamily_link, selected_version, selected_subversion, selected_platform
    ));
}

async fn download_and_extract(download_url: String) {
    let client = NonBlockingClient::new();
    info!("Downloading '{}'", download_url);

    let resp = client
        .get(&download_url)
        .send()
        .await
        .unwrap_or_else(|err| {
            error!("Failed to GET from '{}'", &download_url);
            exit(-1);
        });
    let total_size = resp
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &download_url));
}

fn get_choices_from_html_tags(resp: Response, filters: Vec<String>) -> Vec<String> {
    let body = match resp.text() {
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

    return versions;
}

fn choose(choices: &Vec<String>) -> String {
    let mut page = 0;
    let per_page = 9;
    loop {
        for i in (page * per_page)..((page + 1) * per_page) {
            if i < choices.len() {
                println!("{}: {}", i + 1, choices[i]);
            }
        }

        let more = ((page + 1) * per_page) < choices.len();
        let mut start = 1;
        if more {
            println!("0: More..");
            start = 0;
        }

        let mut choice: u8;

        // Loop forever until user input correct choice.
        loop {
            println!(
                "Please select a with number[{} - {}]: ",
                start,
                choices.len()
            );

            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();

            choice = match input.trim().parse() {
                Ok(choice) => choice,
                Err(err) => {
                    error!("Parsing user input: {}", err);
                    continue;
                }
            };

            if usize::from(choice) > choices.len() {
                error!("Invalid Choice. Try again.");
                continue;
            }

            break;
        }

        if choice == 0 {
            page += 1;
            continue;
        }

        return choices[usize::from(choice) - 1].clone();
    }
}
