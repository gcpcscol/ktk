//! `ktk` is a command line tool to manage multiple kubeconfig files simultaneously in different kitty tabs.
//!
//! It is possible to customize the name and the color of the tabs for each cluster, to search quickly in thousands of namespaces, with a cache file.
//! When `ktk` open a new tab, you go directly to a working directory specific to the cluster and the namespace.
//!
//! `ktk` can easily manage dozens of clusters with thousands of namespaces.
mod config;
mod kube;
mod kubeconfig;
mod ohmyposh;
mod terminal;

use clap::{
    Arg, ArgAction, ValueHint, command, crate_authors, crate_name, crate_version, value_parser,
};
use clap_complete::aot::{Generator, Shell, generate};
use regex::bytes::Regex;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{env, io, process};

use log::{debug, error, info};
use simplelog::*;

fn config_file() -> String {
    match env::var("KTKONFIG") {
        Ok(v) => v,
        Err(_) => {
            let cfd = dirs::config_dir().unwrap().as_path().display().to_string();
            format!("{cfd}/{}.yaml", crate_name!())
        }
    }
}

fn logfile() -> String {
    match env::var("KTKLOG") {
        Ok(v) => v,
        Err(_) => {
            let logdir = dirs::home_dir().unwrap().as_path().display().to_string();
            format!("{logdir}/{}.log", crate_name!())
        }
    }
}

fn clap_command(pns: Vec<String>, pnsinc: Vec<String>) -> clap::Command {
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let after_help: &'static str = color_print::cstr!(
        r#"<bold><green>Examples:</green></bold>
  <dim>$</dim> <bold>ktk kube-system::production</bold>
  <dim>$</dim> <bold>ktk -t -C kube-system</bold>
"#
    );
    let override_usage: &'static str = color_print::cstr!(
        r#"<bold><green>Usage:</green></bold> <bold>ktk</bold> [OPTIONS] [namespace::cluster]"#
    );

    command!() // requires `cargo` feature
        .help_template("\
{before-help}{name} {version}
{author-with-newline}{about-with-newline}
{usage}

{all-args}{after-help}
")
        .before_long_help(format!(
            "{} search for you the good namespace and load it directly in a kitty tab.
    The new tab is open directly in the good working directory.",
            crate_name!()
        ))
        .arg_required_else_help(true)
        .arg(
            Arg::new("namespace")
                .help("Namespace to operate on")
                .required_unless_present_any(["force","evaldir","cluster","completion","list-clusters-colors","list-clusters-names","oh-my-posh-json"])
                .value_hint(ValueHint::Other)
        )
        .arg(
            Arg::new("namespace-in-current-context")
                .value_parser(pnsinc)
                .required(false)
                .hide(true)
        )
        .arg(
            Arg::new("namespace-in-all-context")
                .value_parser(pns)
                .required(false)
                .hide(true)
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .action(ArgAction::Set)
                .help("Sets a custom config file")
                .long_help("Sets a custom config file.\nIt is possible to set the environment variable KTKONFIG to redefine the default config file.")
                .default_value(config_file())
                .value_hint(ValueHint::FilePath)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Force reconstruct cache of namespace")
                .long_help("This option will rebuild the whole cache by requesting all clusters. Each line will contain the namespace name, followed by the cluster name. If a cluster is not available the cache data for it will be deleted."),
        )
        .arg(
            Arg::new("noscan")
                .short('n')
                .long("noscan")
                .action(clap::ArgAction::SetTrue)
                .help("Do not reconstruct cache of namespace")
                .long_help("The cache is automatically rebuilt every \"maxage\" seconds. This option allows you to ignore this value to avoid refreshing the cache.")
                .conflicts_with_all(["force"]),
        )
        .arg(
            Arg::new("cluster")
                .short('C')
                .long("cluster")
                .action(clap::ArgAction::SetTrue)
                .help(format!("Search only in current cluster like kubens (alias kubens=\"{} -t -C\")",crate_name!()))
        )
        .arg(
            Arg::new("list-clusters-colors")
                .short('l')
                .long("list-clusters-colors")
                .action(clap::ArgAction::SetTrue)
                .help("List kube clusters with tabs colors in config file")
                .conflicts_with_all(["list-clusters-names","oh-my-posh-json"]),
        )
        .arg(
            Arg::new("list-clusters-names")
                .short('L')
                .long("list-clusters-names")
                .action(clap::ArgAction::SetTrue)
                .help("List kube clusters names in config file")
                .conflicts_with_all(["list-clusters-colors","oh-my-posh-json"]),
        )
        .arg(
            Arg::new("oh-my-posh-json")
                .short('O')
                .long("oh-my-posh-json")
                .action(clap::ArgAction::SetTrue)
                .help("Update oh-my-posh json config file")
        )
        .arg(
            Arg::new("subfilter")
                .short('s')
                .long("subfilter")
                .action(clap::ArgAction::Set)
                .help("Pre-filter on a subset of value with a regexp")
                .long_help("Defines a pre-filter on a subset of values using a regular expression.\nThe KTKSUBFILTER environment variable can be set to define this pre-filter.")
        )
        .arg(
            Arg::new("wait")
                .short('w')
                .long("wait")
                .action(clap::ArgAction::SetTrue)
                .help("Disable timeout for namespaces search")
                .long_help("Allows to override the timeout value of the config file in order to have temporarily a longer time for the cluster to respond.")
                .conflicts_with_all(["evaldir", "noscan","completion"]),
        )
        .arg(
            Arg::new("tab")
                .short('t')
                .long("tab")
                .action(clap::ArgAction::SetTrue)
                .help("Change namespace without change tab (like kubens)")
                .conflicts_with_all(["evaldir"]),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .action(clap::ArgAction::SetTrue)
                .help("Record debug event in log file")
        )
        .arg(
            Arg::new("evaldir")
                .short('e')
                .long("evaldir")
                .action(clap::ArgAction::SetTrue)
                .help("Force reconstruct cache of namespace")
                .help("Show in stdout workdir of current cluster")
                .long_help("Show in stdout workdir of current cluster.\nUse in your .bahsrc or .zshrc file to automatically load the correct kubeconfig file.")
                .conflicts_with_all(["namespace", "force", "tab", "wait", "noscan","cluster","completion"]),
        )
        .arg(
            Arg::new("completion")
               .long("completion")
               .action(clap::ArgAction::Set)
               .help("Output shell completion code for the specified shell")
               .conflicts_with_all(["namespace"])
               .value_parser(clap::value_parser!(Shell)),
        )
        .version(crate_version!())
        .long_version(
            format!("{}\n{:<12} :  {}\n{:<12} :  {}\n{:<12} :  {}",
                crate_version!(),
                "Authors",
                crate_authors!(),
                "Time",
                build_timestamp,
                "ShortCommit",
                git_hash,
            )
        )
        .author(crate_authors!())
        .after_help(after_help)
        .override_usage(override_usage)
}

fn print_completions<G: Generator>(g: G, cmd: &mut clap::Command) {
    generate(g, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn configlog(activedebug: bool) {
    // Logger
    let conflog = ConfigBuilder::new()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] UTC"
        ))
        .build();

    let mut log_level_term = LevelFilter::Warn;
    let mut log_level_file = LevelFilter::Info;
    if activedebug {
        log_level_term = LevelFilter::Debug;
        log_level_file = LevelFilter::Debug;
    }
    CombinedLogger::init(vec![
        TermLogger::new(
            log_level_term,
            conflog.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            log_level_file,
            conflog,
            OpenOptions::new()
                .create(true) // to allow creating the file, if it doesn't exist
                .append(true) // to not truncate the file, but instead add to it
                .open(logfile())
                .unwrap(),
        ),
    ])
    .unwrap();
}

fn evaldir(conf: &config::Context) {
    // For evaldir option, prompt only environnement variable Kubeconfig
    // and change directory with eval command like this :
    //
    // kubedir=$(ktk --evaldir)
    // if [ "$?" -eq 0 ]; then
    //   eval "$(echo $kubedir)"
    // fi
    let term = terminal::detect();
    let idpath = term.id_path_of_focus_tab();
    debug!("idpath : {:?}", idpath);
    if idpath.is_some() {
        let kubeconfig = format!("{}/{}", conf.kubetmp, idpath.unwrap());
        if !Path::new(&kubeconfig).exists() {
            debug!("file not found : {:?}", kubeconfig);
            process::exit(1)
        }
        let kcf = match kubeconfig::Kubeconfig::new(kubeconfig.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("Error parsing file {kubeconfig}: {e:?}");
                process::exit(6)
            }
        };
        let cluster_context = kcf.cluster_context();
        let namespace_context = kcf.namespace_context();
        let cluster = match conf.cluster_named(&cluster_context) {
            Some(v) => v,
            None => {
                error!(
                    "Unable to find the cluster name {cluster_context} in the configuration file."
                );
                process::exit(7)
            }
        };

        println!(
            "{}",
            kube::ns_workdir(cluster, namespace_context, kubeconfig)
        );
    }
}

fn possible_namespaces_in_context(conf: config::Context, cluster_search: String) -> Vec<String> {
    conf.read_completion_file()
        .split('\n')
        .filter(|s| s.ends_with(format!("{}{}", conf.separator, cluster_search.clone()).as_str()))
        .map(
            |x| match x.strip_suffix(format!("{}{}", conf.separator, cluster_search).as_str()) {
                Some(v) => v.to_string(),
                None => "".to_string(),
            },
        )
        .collect()
}

fn possible_namespaces(conf: config::Context, regexsubfilter: Regex) -> Vec<String> {
    conf.read_completion_file()
        .split('\n')
        .filter(|s| regexsubfilter.captures(s.as_bytes()).is_some())
        .map(|x| x.to_string())
        .collect()
}

fn main() -> Result<(), io::Error> {
    // load clap config
    let matches = clap_command(Vec::new(), Vec::new()).get_matches();

    // logger loading
    if matches.get_flag("debug") {
        configlog(true);
    } else {
        configlog(false);
    }

    // Checking the presence of the configuration file
    let config_path = match matches.get_one::<PathBuf>("config") {
        Some(v) => v,
        None => {
            error!("Config file missing");
            process::exit(51)
        }
    };
    debug!("config_path: {:?}", config_path);
    // Check if config file exist
    if !Path::new(config_path).exists() {
        error!("Config file missing: {}", config_path.display());
        process::exit(52)
    }

    // Load yaml config file
    let conf = config::Context::new(config_path, matches.get_flag("wait"));

    if matches.get_flag("list-clusters-names") {
        conf.list_clusters_names();
        process::exit(0)
    }

    if matches.get_flag("list-clusters-colors") {
        conf.list_clusters_colors();
        process::exit(0)
    }

    if matches.get_flag("oh-my-posh-json") {
        conf.update_ohmyposh_config();
        process::exit(0)
    }

    if matches.get_flag("evaldir") {
        evaldir(&conf);
        process::exit(0)
    }

    let mut term = terminal::detect();
    // Initialize user input namespace
    let namespace_search = match matches.get_one::<String>("namespace") {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let mut cluster_search = "".to_string();

    // Get current cluster context
    if matches.get_flag("cluster") || matches.contains_id("completion") {
        debug!("Get current cluster context");
        match env::var("KUBECONFIG") {
            Ok(kc) => {
                cluster_search = match kubeconfig::Kubeconfig::new(kc.clone()) {
                    Ok(v) => v.cluster_context(),
                    Err(e) => {
                        if !matches.contains_id("completion") {
                            error!("error parsing file {}: {e:?}", kc);
                            process::exit(6);
                        }
                        "".to_string()
                    }
                };
            }
            Err(_) => {
                error!("No kubeconfig");
                if !matches.contains_id("completion") {
                    process::exit(1)
                }
            }
        };
    }

    // Check if the completion file must be update
    if !matches.get_flag("noscan")
        && (conf.completion_file_older_than_maxage()
            || conf.completion_file_older_than_config()
            || matches.get_flag("force"))
    {
        debug!("Update completion file {}", conf.completion_filename);
        conf.update_completion_file();
    }

    let subfilter_env = match env::var("KTKSUBFILTER") {
        Ok(v) => v,
        Err(_) => ".*".to_string(),
    };
    debug!("Env KTKSUBFILTER {}", subfilter_env);
    let subfilter = matches
        .get_one::<String>("subfilter")
        .unwrap_or(&subfilter_env);
    debug!("subfilter {}", subfilter);
    let regexsubfilter = Regex::new(subfilter).unwrap();
    debug!("regexsubfilter {}", regexsubfilter);

    if let Some(generator) = matches.get_one::<Shell>("completion").copied() {
        let pns = possible_namespaces(conf.clone(), regexsubfilter);
        let pnsinc = possible_namespaces_in_context(conf.clone(), cluster_search.clone());
        let mut cmd = clap_command(pns, pnsinc);
        print_completions(generator, &mut cmd);
        process::exit(0)
    }

    // Show fuzzy search to choose the namespace
    // In kubens mode, we only display the namespace, not the cluster name
    let choice = if matches.get_flag("cluster") {
        format!(
            "{}{}{}",
            kube::selectable_list(
                possible_namespaces_in_context(conf.clone(), cluster_search.clone()),
                Some(namespace_search),
            ),
            conf.separator,
            cluster_search
        )
    } else {
        kube::selectable_list(
            possible_namespaces(conf.clone(), regexsubfilter),
            Some(namespace_search),
        )
    };
    if choice.is_empty() {
        debug!("Empty choice");
        process::exit(130);
    }
    // Check if the tab doesn't already exist.
    // If it exists, go to tab,
    // otherwise create a new one.
    let tab_name = format!("{}{}", conf.tabprefix, &choice);
    if term.focus_tab_name(&tab_name) {
        info!("go to {choice}");
    } else {
        info!("launch {choice}");
        // Get namespace arg
        let s: Vec<&str> = choice.split(&conf.separator).collect();
        if s.is_empty() {
            process::exit(0);
        }
        let namespace = s[0];
        let mut clustername = "".to_string();
        if s.len() == 1 && matches.get_flag("cluster") {
            clustername = cluster_search
        }
        if s.len() == 2 {
            clustername = s[1].to_string();
        }
        if !matches.get_flag("tab") {
            debug!("create new tab => {tab_name}");
            term.create_new_tab(&tab_name);
        } else {
            debug!("change tab title => {tab_name}");
            term.change_tab_title(&tab_name);
        }
        let cl = conf.cluster_named(clustername.as_str()).unwrap();
        debug!("cluster name => {}", clustername.as_str());
        let destkubeconfig = format!("{}/{}", conf.kubetmp, term.identifier());
        debug!("destination directory for kubeconfig files => {destkubeconfig}");
        term.change_tab_color(cl.tabcolor.clone());
        println!();
        let mut kcf = match kubeconfig::Kubeconfig::new(cl.kubeconfig_path.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("error parsing file {}: {e:?}", cl.kubeconfig_path);
                process::exit(6)
            }
        };
        debug!("change kube context => {}", namespace);
        kcf.change_context(namespace.to_string());
        match term.id_of_tab_name(&tab_name) {
            Some(tab_id) => {
                debug!("tab_id => {}", tab_id);
                debug!("write new kubeconfig in {}/{}", destkubeconfig, tab_id);
                kcf.write(destkubeconfig, tab_id);
                term.focus_tab_name(&tab_name);
            }
            None => {
                let tab_id = term.id_of_focus_tab().unwrap();
                debug!("tab_id => {}", tab_id);
                debug!("write new kubeconfig in {}/{}", destkubeconfig, tab_id);
                kcf.write(destkubeconfig, tab_id);
            }
        }
    }

    Ok(())
}
