mod config;
mod kitty;
mod kube;
mod kubeconfig;

use clap::{arg, command, crate_authors, crate_name, crate_version, value_parser, Arg, ArgAction};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{env, io, process};

use log::{error, info};
use simplelog::*;

fn config_file() -> String {
    match env::var("KTKONFIG") {
        Ok(v) => v,
        Err(_) => {
            let cfd = dirs::config_dir().unwrap().as_path().display().to_string();
            format!("{cfd}/ktk.yaml")
        }
    }
}

fn logfile() -> String {
    match env::var("KTKLOG") {
        Ok(v) => v,
        Err(_) => {
            let logdir = dirs::home_dir().unwrap().as_path().display().to_string();
            format!("{logdir}/ktk.log")
        }
    }
}

fn main() -> Result<(), io::Error> {
    let matches = command!() // requires `cargo` feature
        .before_long_help(format!(
            "{} search for you the good namespace and load it directly in a kitty tab.
    The new tab is open directly in the good working directory.",
            crate_name!()
        ))
        .arg_required_else_help(true)
        .arg(arg!(
            [namespace] "Namespace to operate on"
        )
            .required_unless_present_any(["force","evaldir","cluster"])
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
                .help("Force reconstruct cache of namespace")
                .help("Don't reconstruct cache of namespace")
                .long_help("The cache is automatically rebuilt every \"maxage\" seconds. This option allows you to ignore this value to avoid refreshing the cache.")
                .conflicts_with_all(["force"]),
        )
        .arg(
            Arg::new("cluster")
                .short('C')
                .long("cluster")
                .action(clap::ArgAction::SetTrue)
                .help("Search only in current cluster (like kubens)")
        )
        .arg(
            Arg::new("wait")
                .short('w')
                .long("wait")
                .action(clap::ArgAction::SetTrue)
                .help("Force reconstruct cache of namespace")
                .help("disable timeout for namespaces search")
                .long_help("Allows to override the timeout value of the config file in order to have temporarily a longer time for the cluster to respond.")
                .conflicts_with_all(["evaldir", "noscan"]),
        )
        .arg(
            Arg::new("tab")
                .short('t')
                .long("tab")
                .action(clap::ArgAction::SetTrue)
                .help("Force reconstruct cache of namespace")
                .help("Change namespace without change tab (like kubens)")
        )
        .arg(
            Arg::new("evaldir")
                .short('e')
                .long("evaldir")
                .action(clap::ArgAction::SetTrue)
                .help("Force reconstruct cache of namespace")
                .help("Show in stdout workdir of current cluster")
                .long_help("Show in stdout workdir of current cluster.\nUse in your .bahsrc or .zshrc file to automatically load the correct kubeconfig file.")
                .conflicts_with_all(["namespace", "force", "tab", "wait", "noscan","cluster"]),
        )
        .version(crate_version!())
        .long_version(format!("{}\n{}", crate_version!(), crate_authors!()))
        .author(crate_authors!())
        .get_matches();

    // Logger
    let conflog = ConfigBuilder::new()
        .set_time_to_local(true)
        .set_time_format_str("%Y-%m-%d %H:%M:%S")
        .build();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            conflog.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            conflog,
            OpenOptions::new()
                .create(true) // to allow creating the file, if it doesn't exist
                .append(true) // to not truncate the file, but instead add to it
                .open(logfile())
                .unwrap(),
        ),
    ])
    .unwrap();

    let config_path = match matches.get_one::<PathBuf>("config") {
        Some(v) => v,
        None => {
            error!("Config file missing");
            process::exit(51)
        }
    };
    // Check if config file exist
    if !Path::new(config_path).exists() {
        error!("Config file missing: {}", config_path.display());
        process::exit(52)
    }

    // Load yaml config file
    let conf = config::Context::new(config_path, matches.get_flag("wait"));

    // Load kitty context (kitty @ls)
    let mut k: kitty::Context;
    if env::var("KITTY_WINDOW_ID").is_ok() {
        k = kitty::Context::new();
    } else {
        if !matches.get_flag("evaldir") {
            error!("This not a kitty terminal");
        }
        process::exit(5)
    }

    // For evaldir option, prompt only environnement variable Kubeconfig
    // and change directory with eval command like this :
    //
    // kubedir=$(ktk --evaldir)
    // if [ "$?" -eq 0 ]; then
    //   eval "$(echo $kubedir)"
    // fi

    if matches.get_flag("evaldir") {
        let idpath = k.id_path_of_focus_tab();
        if idpath.is_some() {
            let kubeconfig = format!("{}/{}", conf.kubetmp, idpath.unwrap());
            if !Path::new(&kubeconfig).exists() {
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
            let cluster = match conf.cluster_by_name(&cluster_context) {
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
        process::exit(0)
    }

    // Initialize user input namespace
    let namespace_search = match matches.get_one::<String>("namespace") {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let mut cluster_search = "".to_string();

    // Get current cluster context
    if matches.get_flag("cluster") {
        let kc = match env::var("KUBECONFIG") {
            Ok(v) => v,
            Err(_) => {
                println!("No kubeconfig");
                process::exit(1)
            }
        };

        let kcf = match kubeconfig::Kubeconfig::new(kc.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("error parsing file {}: {e:?}", kc);
                process::exit(6)
            }
        };
        cluster_search = kcf.cluster_context();
    }

    // Check if the completion file must be update
    if !matches.get_flag("noscan")
        && (conf.completion_file_older_than_maxage()
            || conf.completion_file_older_than_config()
            || matches.get_flag("force"))
    {
        conf.update_completion_file();
    }
    // Show fuzzy search to choose the namespace
    // In kubens mode, we only display the namespace, not the cluster name
    let mut choice = kube::selectable_list(
        conf.read_completion_file()
            .split('\n')
            .filter(|s| {
                if matches.get_flag("cluster") {
                    s.ends_with(format!("{}{}", conf.separator, cluster_search.clone()).as_str())
                } else {
                    true
                }
            })
            .map(|x| {
                if !matches.get_flag("cluster") {
                    x.to_string()
                } else {
                    match x.strip_suffix(
                        format!("{}{}", conf.separator, cluster_search.clone()).as_str(),
                    ) {
                        Some(v) => v.to_string(),
                        None => "".to_string(),
                    }
                }
            })
            .collect(),
        Some(namespace_search.as_str()),
    );
    if choice.is_empty() {
        process::exit(0);
    }
    if matches.get_flag("cluster") {
        choice = format!("{}{}{}", choice, conf.separator, cluster_search);
    }
    // Check if the tab doesn't already exist.
    // If it exists, go to tab,
    // otherwise create a new one.
    let tab = format!("{}{}", conf.tabprefix, &choice);
    if let Some(idwin) = k.id_window_with_tab_title(&tab) {
        info!("go to {choice}");
        k.focus_window_id(idwin)
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
            k.launch_shell_in_new_tab_name(&tab);
        } else {
            k.set_tab_title(&tab);
        }
        let cl = conf.cluster_by_name(clustername.as_str()).unwrap();
        let destkubeconfig = format!("{}/{}", conf.kubetmp, k.platform_window_id());
        k.set_tab_color(cl.tabcolor.clone());
        println!();
        // let mut kcf = kubeconfig::Kubeconfig::new(cl.kubeconfig.clone());
        let mut kcf = match kubeconfig::Kubeconfig::new(cl.kubeconfig.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("error parsing file {}: {e:?}", cl.kubeconfig);
                process::exit(6)
            }
        };
        kcf.change_context(namespace.to_string());
        kcf.write(destkubeconfig, k.id_of_focus_tab().unwrap().to_string());
    }

    Ok(())
}
