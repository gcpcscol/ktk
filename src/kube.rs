use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, ListParams},
    config::{KubeConfigOptions, Kubeconfig},
    Client, Config,
};
use log::{info, warn};
use skim::prelude::*;
use std::{io::Cursor, path::Path, process, sync::mpsc, thread, time::Duration};

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub name: String,            // cluster name
    pub kubeconfig_path: String, // kubeconfig path/file
    pub workdir: String,         // cluster working directory
    pub prefixns: String,        // prefix before the name of the working directory
    pub disabled: bool,          // cluster is disabled
    pub tabcolor: crate::terminal::kitty::Tabcolor,
    pub timeout: u32, // maximum time to retrieve the list of namespaces
}

pub fn ns_workdir(cluster: &Cluster, namespace: String, kubeconfig: String) -> String {
    if cluster.prefixns.is_empty()
        || namespace.get(..cluster.prefixns.len()).unwrap_or("") != cluster.prefixns
    {
        let testpath = format!("{}/{}", cluster.workdir, namespace);
        if Path::new(&testpath).exists() {
            format!(
                "export KUBECONFIG={} && cd {}/{}",
                kubeconfig, cluster.workdir, namespace
            )
        } else {
            format!("export KUBECONFIG={} && cd {}", kubeconfig, cluster.workdir)
        }
    } else {
        let nsdir = namespace.get(cluster.prefixns.len()..).unwrap();
        let testpath = format!("{}/{}", cluster.workdir, nsdir);
        if Path::new(&testpath).exists() {
            format!(
                "export KUBECONFIG={} && cd {}/{}",
                kubeconfig, cluster.workdir, nsdir
            )
        } else {
            format!("export KUBECONFIG={kubeconfig} && cd {}", cluster.workdir)
        }
    }
}

pub fn get_kubeconfig_option(kubeconfig: Kubeconfig) -> Option<KubeConfigOptions> {
    for ct in kubeconfig.contexts.iter() {
        if Some(ct.name.clone()) == kubeconfig.current_context {
            match ct.context.clone() {
                Some(context) => {
                    return Some(KubeConfigOptions {
                        context: kubeconfig.current_context,
                        cluster: Some(context.cluster),
                        user: Some(context.user),
                    });
                }
                None => return None,
            }
        }
    }
    None
}

#[tokio::main]
pub async fn get_namespaces(kubeconfig: Kubeconfig, sep: String, timeout: u32) -> Vec<String> {
    match get_kubeconfig_option(kubeconfig.clone()) {
        Some(kubeopt) => {
            let config = match Config::from_custom_kubeconfig(kubeconfig, &kubeopt).await {
                Ok(mut c) => {
                    c.connect_timeout = Some(Duration::from_secs(timeout.into()));
                    c
                }
                Err(e) => {
                    warn!("{}", e);
                    return Vec::new();
                }
            };
            let client = match Client::try_from(config) {
                Ok(c) => c,
                Err(e) => {
                    warn!("{}", e);
                    return Vec::new();
                }
            };
            let namespaces: Api<Namespace> = Api::all(client);

            let cluster_name = match kubeopt.cluster {
                Some(c) => c,
                None => return Vec::new(),
            };
            let lp = ListParams::default().timeout(timeout).match_any();
            match namespaces.list(&lp).await {
                Ok(n) => {
                    let ns: Vec<String> = n
                        .items
                        .iter()
                        .map(|name| match name.metadata.name.clone() {
                            Some(ns) => format!("{}{}{}", ns, sep, cluster_name),
                            None => "".to_string(),
                        })
                        .collect();
                    return ns;
                }
                Err(_) => {
                    warn!("{} is unreachable", cluster_name);
                    return Vec::new();
                }
            }
        }
        None => return Vec::new(),
    }
}

pub fn get_all_ns(clusters: Vec<Cluster>, sep: String) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    let mut nbcl = 0;
    let mut result = Vec::new();
    for cl in clusters {
        if !cl.disabled {
            nbcl += 1;
            let tx1 = tx.clone();
            let s = sep.clone();
            let t = cl.timeout;
            thread::spawn(move || {
                let _ = match Kubeconfig::read_from(cl.kubeconfig_path) {
                    Ok(k) => tx1.send(get_namespaces(k, s, t)),
                    Err(e) => {
                        warn!("{}", e);
                        tx1.send(Vec::new())
                    }
                };
            });
        }
    }
    thread::spawn(move || {
        let _ = tx.send(vec!["".to_string()]);
    });
    for rec in rx {
        result.extend(rec);
    }
    result.sort();
    info!("{} namespaces found in {} clusters", result.len(), nbcl);
    result
}

pub fn selectable_list(input: Vec<String>, query: Option<&str>) -> String {
    if input.contains(&query.unwrap().to_string()) {
        return query.unwrap().to_string();
    };
    let input: Vec<String> = input.into_iter().rev().collect();
    let options = SkimOptionsBuilder::default()
        .multi(false)
        .query(query)
        .select1(false)
        .build()
        .unwrap();
    let item_reader = SkimItemReader::default();

    let items = item_reader.of_bufread(Cursor::new(input.join("\n")));
    let selected_items = Skim::run_with(&options, Some(items))
        .map(|out| match out.final_key {
            Key::Enter => out.selected_items,
            _ => Vec::new(),
        })
        .unwrap_or_default();

    if selected_items.is_empty() {
        warn!("Empty Choice");
        process::exit(1);
    }

    selected_items[0].output().to_string()
}

#[cfg(test)]
mod tests {
    use crate::{kube::ns_workdir, terminal::kitty::Tabcolor};

    use super::Cluster;

    #[test]
    fn test_ns_workdir_path_not_exist() {
        let namespace = "test-mynamespace".to_string();
        let name = "test".to_string();
        let cluster_kubeconfig_path = "/path/kubeconfig/cluster".to_string();
        let workdir = "/path/workdir/test".to_string();
        let prefixns = "test-".to_string();
        let disabled = false;
        let tabcolor = Tabcolor {
            active_bg: "NONE".to_string(),
            inactive_bg: "NONE".to_string(),
            active_fg: "NONE".to_string(),
            inactive_fg: "NONE".to_string(),
        };
        let timeout = 5;
        let cluster = Cluster {
            name,
            kubeconfig_path: cluster_kubeconfig_path,
            workdir,
            prefixns,
            disabled,
            tabcolor,
            timeout,
        };
        let kubeconfig = "/tmp/path/kitty/42".to_string();
        let result = ns_workdir(&cluster, namespace, kubeconfig);
        assert_eq!(
            result,
            "export KUBECONFIG=/tmp/path/kitty/42 && cd /path/workdir/test".to_string()
        );
    }

    #[test]
    fn test_ns_workdir_path_exist() {
        let namespace = "test-fonts".to_string();
        let name = "test".to_string();
        let cluster_kubeconfig_path = "/path/kubeconfig/cluster".to_string();
        let workdir = "/usr/share".to_string();
        let prefixns = "test-".to_string();
        let disabled = false;
        let tabcolor = Tabcolor {
            active_bg: "NONE".to_string(),
            inactive_bg: "NONE".to_string(),
            active_fg: "NONE".to_string(),
            inactive_fg: "NONE".to_string(),
        };
        let timeout = 5;
        let cluster = Cluster {
            name,
            kubeconfig_path: cluster_kubeconfig_path,
            workdir,
            prefixns,
            disabled,
            tabcolor,
            timeout,
        };
        let kubeconfig = "/tmp/path/kitty/42".to_string();
        let result = ns_workdir(&cluster, namespace, kubeconfig);
        assert_eq!(
            result,
            "export KUBECONFIG=/tmp/path/kitty/42 && cd /usr/share/fonts".to_string()
        );
    }

    #[test]
    fn test_ns_workdir_path_exist_no_prefix() {
        let namespace = "fonts".to_string();
        let name = "test".to_string();
        let cluster_kubeconfig_path = "/path/kubeconfig/cluster".to_string();
        let workdir = "/usr/share".to_string();
        let prefixns = "".to_string();
        let disabled = false;
        let tabcolor = Tabcolor {
            active_bg: "NONE".to_string(),
            inactive_bg: "NONE".to_string(),
            active_fg: "NONE".to_string(),
            inactive_fg: "NONE".to_string(),
        };
        let timeout = 5;
        let cluster = Cluster {
            name,
            kubeconfig_path: cluster_kubeconfig_path,
            workdir,
            prefixns,
            disabled,
            tabcolor,
            timeout,
        };
        let kubeconfig = "/tmp/path/kitty/42".to_string();
        let result = ns_workdir(&cluster, namespace, kubeconfig);
        assert_eq!(
            result,
            "export KUBECONFIG=/tmp/path/kitty/42 && cd /usr/share/fonts".to_string()
        );
    }
}
