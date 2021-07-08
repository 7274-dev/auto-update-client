extern crate tokio;
extern crate clap;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

async fn deployment(matches: &ArgMatches<'_>) {
    // safe to unwrap, has a default value
    let commit_hash = matches.value_of("commit").unwrap();
}

async fn logs(matches: &ArgMatches<'_>) {

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
                                                .default_value("latest")
                                                .long("commit"))
                                            .arg(Arg::with_name("server")
                                                .short("s")
                                                .long("server")
                                                .value_name("URL")
                                                .help("The URL of the server you want to interact with")
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
                                                .required(true)))
                        .get_matches();

    match matches.subcommand() {
        ("deployment", Some(matches)) => deployment(matches).await,
        ("logs", Some(matches)) => logs(matches).await,
        _ => ()
    };
}
