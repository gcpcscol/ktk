mod config;
mod kitty;
mod kube;
mod kubeconfig;

use clap::{arg, command, crate_authors, crate_name, crate_version, value_parser};
use std::path::{Path, PathBuf};
use std::{env, io, process};

fn config_file() -> String {
    let cfd = dirs::config_dir().unwrap().as_path().display().to_string();
    format!("{cfd}/ktk.yaml")
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
        ))
        .arg(
            arg!(
                -c --config <FILE> "Sets a custom config file"
            )
            .default_value(config_file())
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(
            -f --force ... "Force reconstruct cache of namespace"
        ))
        .arg(arg!(
            -t --tab ... "Change namespace without change tab (like kubens)"
        ))
        .arg(
            arg!(
                -n --noscan ... "Don't reconstruct cache of namespace"
            )
            .conflicts_with_all(["force"]),
        )
        .arg(
            arg!(
            -e --evaldir ... "Show in stdout workdir of current cluster"
                )
            .conflicts_with_all(["namespace", "force", "tab"]),
        )
        .version(crate_version!())
        .long_version(format!("{}\n{}", crate_version!(), crate_authors!()))
        .author(crate_authors!())
        .get_matches();

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        // Check if config file exist
        if !Path::new(config_path).exists() {
            println!("Config file missing: {}", config_path.display());
            process::exit(5)
        }

        // Load yaml config file
        let conf = config::Context::new(config_path);
        // Load kitty context (kitty @ls)
        let mut k: kitty::Context;
        if env::var("KITTY_WINDOW_ID").is_ok() {
            k = kitty::Context::new();
        } else {
            println!("This not a kitty terminal");
            process::exit(5)
        }

        // For evaldir option, prompt only environnement variable Kubeconfig
        // and change directory with eval command like this :
        //
        // kubedir=$(ktk --evaldir)
        // if [ "$?" -eq 0 ]; then
        //   eval "$(echo $kubedir)"
        // fi

        if matches.get_count("evaldir") >= 1 {
            let idpath = k.id_path_of_focus_tab();
            if idpath.is_some() {
                let kubeconfig = format!("{}/{}", conf.kubetmp, idpath.unwrap());
                if !Path::new(&kubeconfig).exists() {
                    process::exit(1)
                }
                let kcf = kubeconfig::Kubeconfig::new(kubeconfig.clone());
                let cluster_context = kcf.cluster_context();
                let namespace_context = kcf.namespace_context();
                let cluster = conf.cluster_by_name(&cluster_context).unwrap();

                println!(
                    "{}",
                    kube::ns_workdir(cluster, namespace_context, kubeconfig)
                );
            }
            process::exit(0)
        }

        // Check if the completion file must be update
        if matches.get_count("noscan") == 0
            && (conf.completion_file_older_than_maxage() || matches.get_count("force") >= 1)
        {
            conf.update_completion_file();
            if matches.get_one::<String>("namespace").is_none() {
                process::exit(0)
            }
        }

        // Show fuzzy search to choose the namespace
        let choice = kube::selectable_list(
            conf.read_completion_file()
                .split('\n')
                .map(|x| x.to_string())
                .collect(),
            matches.get_one::<String>("namespace").map(|x| &**x),
        );

        // Check if the tab doesn't already exist.
        // If it exists, go to tab,
        // otherwise create a new one.
        let tab = format!("{}{}", conf.tabprefix, &choice);
        let idtab = k.id_tab_with_title(&tab);
        if let Some(v) = idtab {
            println!("go to {choice}");
            k.focus_tab_id(v)
        } else {
            println!("launch {choice}");
            // Get namespace arg
            let s: Vec<&str> = choice.split(&conf.separator).collect();
            let namespace = s[0];
            if matches.get_count("tab") == 0 {
                k.launch_shell_in_new_tab_name(&tab);
            } else {
                k.set_tab_title(&tab);
            }
            let cl = conf.cluster_by_name(s[1]).unwrap();
            let destkubeconfig = format!("{}/{}", conf.kubetmp, k.platform_window_id().unwrap());
            k.set_tab_color(cl.tabcolor.clone());
            println!();
            let mut kcf = kubeconfig::Kubeconfig::new(cl.kubeconfig.clone());
            kcf.change_context(namespace.to_string());
            kcf.write(destkubeconfig, k.id_of_focus_tab().unwrap().to_string());
        }
    }

    Ok(())
}
