extern crate tokio;
extern crate clap;
extern crate chrono;
extern crate colored;

use std::{process::{Command, exit}};

use chrono::NaiveDateTime;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use device_query::{DeviceState, Keycode};
use git2::{Commit, Oid, Repository, Sort};
use tempdir::TempDir;
use colored::*;
use terminal_size::{Height, terminal_size};
use serde::{Serialize, Deserialize};

const REPO_URL: &str = "https://github.com/7274-dev/AdventnaVyzva-GlobalBackend";

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    response: String,
    status_code: u16
}

async fn choose_commit() -> Oid {
    let tmp_dir = TempDir::new("repo").unwrap_or_else(|e| panic!("Failed to create temp dir: {}", e));
    let repo = match Repository::clone(REPO_URL, tmp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed while cloning repo: {}", e)
    };

    let mut revwalk = match repo.revwalk() {
        Ok(c) => c,
        Err(e) => panic!("Failed to get revwalk: {}", e)
    };

    revwalk.set_sorting(Sort::TIME).unwrap();
    revwalk.push_head().unwrap();

    let mut all_commits: Vec<Commit> = Vec::new();
    
    revwalk.for_each(|i| {
        let oid = i.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        
        all_commits.push(commit);
    });
    
    let replacer = gh_emoji::Replacer::new();
    let device_state = DeviceState::new();
    
    let mut should_print_commits = true;
    let mut keys_held_previous_iteration: Vec<Keycode> = Vec::new();
    let mut commit_index = 0;
    
    #[cfg(unix)]
    match Command::new("stty")
        .arg("-echo")
        .spawn() {
        Ok(_) => (),
        Err(_) => ()
    
    };

    loop {
        let keys: Vec<Keycode> = device_state.query_keymap();
        let pressed_keys: Vec<Keycode> = keys.clone().into_iter().filter(|x| !keys_held_previous_iteration.contains(x)).collect();
        
        keys_held_previous_iteration = keys.clone();

        if pressed_keys.contains(&Keycode::Up) && commit_index != 0 {
            commit_index -= 1;
            should_print_commits = true;
        }
        else if pressed_keys.contains(&Keycode::Down) {
            commit_index += 1;
            if commit_index == all_commits.len() {
                commit_index = all_commits.len() - 1;
            }
            should_print_commits = true;
        }
        else if pressed_keys.contains(&Keycode::Enter) {
            break;
        }
        
        if should_print_commits {
            let mut output = String::new();

            for i in commit_index..commit_index + 5 {
                if i >= all_commits.len() {
                    for _ in commit_index + (i - commit_index)..commit_index + 5 {
                        output += "\n";
                    }
                    break
                }
                let commit = all_commits.get(i).unwrap();
                
                let message = replacer.replace_all(commit.message().unwrap());
                let time = commit.time();
                let date = NaiveDateTime::from_timestamp(time.seconds(), 0);
                let commit_hash = &commit.id().to_string()[..7];

                let s = format!("  [{}] {}- {}\n", commit_hash, date.to_string(), message)
                                                .replace("\n\n", "\n")
                                                .replace("\n", "\n                                   ");
                
                
                if s.ends_with("\n                                   ") {
                    if commit_index == i {
                        output += &format!("->{}\n", &s[..s.len() - 36].green());
                    }
                    else {
                        output += &format!("  {}\n", &s[..s.len() - 36]);
                    }
                    
                }
                else {
                    if commit_index == i {
                        output += &format!("->{}\n", s.green());
                    }
                    else {
                        output += &format!("  {}\n", s);
                    }
                    
                }
            }
            let term_size = terminal_size();
            let mut padding = String::new();
            if let Some((_, Height(height))) = term_size {
                let output_length = output.matches("\n").count();
                
                let padding_size = ((height as i64) - output_length as i64).abs();
                for _ in 0..padding_size {
                    padding += "\n";
                }
            }
            else {
                panic!("Failed to get terminal height");
            }

            print!("\n\n\n{}{}", padding, output);
            println!("{}", "\nUse UP and DOWN keys to move your selection, press ENTER to confirm your selection.".red());
            should_print_commits = false;
        }
        
    }

    #[cfg(unix)]
    match Command::new("stty")
        .arg("echo")
        .spawn() {
        Ok(_) => (),
        Err(_) => ()
    
    };
    
    all_commits.get(commit_index).unwrap().id()
}

async fn deployment(matches: &ArgMatches<'_>) {
    // password and server are required, safe to unwrap
    let password = matches.value_of("password").unwrap();
    let server = matches.value_of("server").unwrap();

    let client = reqwest::Client::new();
    if matches.value_of("action").unwrap() == "start" {
        let commit_hash = matches.value_of("commit").unwrap_or("none");

        let commit: Oid;
        if commit_hash == "none" {
            commit = choose_commit().await;
        }
        else {
            let tmp_dir = TempDir::new("repo").unwrap_or_else(|e| panic!("Failed to create temp dir: {}", e));
            let repo = match Repository::clone(REPO_URL, tmp_dir.path()) {
                Ok(repo) => repo,
                Err(e) => panic!("Failed while cloning repo: {}", e)
            };

            match repo.find_commit(Oid::from_str(commit_hash).unwrap()) {
                Ok(o) => { commit = o.id() },
                Err(_) => {
                    println!("{} is not a correct commit hash", commit_hash);
                    exit(1);
                }
            };
        }

        println!("Deploying commit {}...", &commit.to_string()[..7]);

        let mut request_url = String::new();
        request_url += "http://";
        request_url += server;
        request_url += "/deploy/";
        request_url += &commit.to_string();

        println!("Sending request... (if everything is correct, the project is building right now)");

        let response = client.post(request_url)
                                        .query(&[("password", password)])
                                        .send().await;

        match response {
            Ok(r) => {
                let response_struct: Response = r.json().await
                                                        .unwrap_or_else(|e| {
                                                            println!("Error while deserializing response: {}", e);
                                                            exit(1);
                                                        });
                let response: &str = &response_struct.response.to_owned();

                match response {
                    "Incorrect Password!" => {
                        println!("Incorrect password!");
                        exit(1);
                    },
                    "Bad commit id" => {
                        println!("Fatal error 1! (this should never happen)");
                        exit(1);
                    }
                    "Error!" => {
                        println!("Build failed, this commit (or an earlier one) is deployed or there was another error during deployment!");
                        exit(1);
                    }
                    "Successfully deployed commit!" => {
                        println!("Deployment succeeded!");
                        exit(0);
                    }
                    _ => {
                        println!("Fatal error 2! (this should never happen)");
                        exit(1);
                    }
                };
                
            },
            Err(e) => {
                println!("Error while making a request: {}", e);
                exit(1);
            } 
        }
    }
    else {
        println!("Stopping current deployment...");

        let mut request_url = String::new();
        request_url += "http://";
        request_url += server;
        request_url += "/stop";

        let response = client.post(request_url)
                .query(&[("password", password)])
                .send().await;
        
        match response {
            Ok(r) => {
                let response_struct: Response = r.json().await.unwrap_or_else(|e| {
                    println!("Error while deserializing response: {}", e);
                    exit(1);
                });

                let response: &str = &response_struct.response.to_owned();
                match response {
                    "Incorrect Password!" => {
                        println!("Incorrect password!");
                        exit(1);
                    },
                    "Stopped current deployment!" => {
                        println!("Successfully stopped current deployment!");
                        exit(0);
                    },
                    "Failed to stop current deployment!" => {
                        println!("There was an error while stopping your deployment!");
                        exit(1);
                    }
                    "Nothing currently deployed!" => {
                        println!("There is nothing currently deployed!");
                        exit(0);
                    },
                    _ => ()
                }

            }
            Err(e) => {
                println!("Error while making a request: {}", e);
                exit(1);
            }
        }

    }
}

async fn logs(matches: &ArgMatches<'_>) {
    // password and server are required, safe to unwrap
    let password = matches.value_of("password").unwrap();
    let server = matches.value_of("server").unwrap();

    let mut request_url = String::new();
    request_url += "http://";
    request_url += server;
    request_url += "/logs";
    
    let client = reqwest::Client::new();
    let response = client.get(request_url)
            .query(&[("password", password)])
            .send().await;

    match response {
        Ok(r) => {
            let response_struct: Response = r.json().await.unwrap_or_else(|e| {
                println!("Error while deserializing response: {}", e);
                exit(1);
            });

            let response: &str = &response_struct.response.to_owned();
            match response {
                "Incorrect password!" => {
                    println!("Incorrect password!");
                    exit(1);
                }
                "Nothing currently deployed!" => {
                    println!("There is nothing currently deployed!");
                    exit(0);
                },
                _ => {
                    println!("{}", response);
                }
            }
        },
        Err(e) => {
            println!("Error while making a request: {}", e);
            exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let matches = App::new("auto-update-client")
                        .version("1.0")
                        .author("7274-dev")
                        .about("easily deploy from your terminal")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .subcommand(SubCommand::with_name("deployment")
                                            .about("manage deployment")
                                            .arg(Arg::with_name("action")
                                                    .index(1)
                                                    .required(true)
                                                    .possible_values(&["start", "stop"]))
                                            .arg(Arg::with_name("commit")
                                                .short("c")
                                                .value_name("COMMIT_HASH")
                                                .help("Hash of the commit you want to deploy")
                                                .takes_value(true)
                                                .long("commit"))
                                            .arg(Arg::with_name("server")
                                                .short("s")
                                                .long("server")
                                                .value_name("URL")
                                                .help("The URL of the server you want to interact with")
                                                .takes_value(true)
                                                .required(true))
                                            .arg(Arg::with_name("password")
                                                    .short("p")
                                                    .long("password")
                                                    .takes_value(true)
                                                    .required(true)))                        
                        .subcommand(SubCommand::with_name("logs")
                                            .about("get logs")
                                            .arg(Arg::with_name("server")
                                                .short("s")
                                                .long("server")
                                                .value_name("URL")
                                                .help("The URL of the server you want to interact with")
                                                .takes_value(true)
                                                .required(true))
                                            .arg(Arg::with_name("password")
                                                .short("p")
                                                .long("password")
                                                .takes_value(true)
                                                .required(true)))
                        .get_matches();

    match matches.subcommand() {
        ("deployment", Some(matches)) => deployment(matches).await,
        ("logs", Some(matches)) => logs(matches).await,
        _ => ()
    };
}
