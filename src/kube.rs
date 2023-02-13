use log::{info, warn};
use skim::prelude::*;
use std::{
    io::Cursor,
    io::Read,
    path::Path,
    process,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub name: String,       // cluster name
    pub kubeconfig: String, // kubeconfig path/file
    pub workdir: String,    // cluster working directory
    pub prefixns: String,   // prefix before the name of the working directory
    pub disabled: bool,     // cluster is disabled
    pub tabcolor: crate::kitty::Tabcolor,
    pub timeout: u64, // maximum time to retrieve the list of namespaces
}

#[allow(dead_code)]
pub fn ns_workdir(cluster: &Cluster, namespace: String, kubeconfig: String) -> String {
    if cluster.prefixns.is_empty()
        || namespace.get(..cluster.prefixns.len()).unwrap() != cluster.prefixns
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

pub fn get_namespaces(cl: Cluster, sep: String) -> Vec<String> {
    let mut child = Command::new("kubectl")
        .arg(format!("--kubeconfig={}", cl.kubeconfig))
        .arg("get")
        .arg("namespace")
        .arg("-o=custom-columns=Name:.metadata.name")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let timeout = Duration::from_secs(cl.timeout);
    let _status_code = match child.wait_timeout(timeout).unwrap() {
        Some(status) => status.code(),
        None => {
            // child hasn't exited yet
            warn!("Unable to contact the cluster {}", cl.name);
            child.kill().unwrap();
            child.wait().unwrap().code()
        }
    };
    let mut s = String::new();
    child.stdout.unwrap().read_to_string(&mut s).unwrap();

    s.lines()
        .skip(1)
        .map(ToOwned::to_owned)
        .map(|x| format!("{}{}{}", x, sep, cl.name))
        .collect()
}

#[allow(unused_must_use)]
pub fn get_all_ns(clusters: Vec<Cluster>) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    let mut nbcl = 0;
    let mut result = Vec::new();
    for cl in clusters {
        if !cl.disabled {
            nbcl += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let ns = get_namespaces(cl, "::".to_string());
                tx1.send(ns);
            });
        }
    }
    thread::spawn(move || {
        tx.send(vec!["".to_string()]);
    });
    for rec in rx {
        result.extend(rec);
    }
    info!("{} namespaces found in {} clusters", result.len(), nbcl);
    result
}

#[allow(dead_code)]
pub fn get_current_context() -> String {
    let output = Command::new("kubectl")
        .args(["config", "current-context"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_owned()
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
    use crate::{kitty::Tabcolor, kube::ns_workdir};

    use super::Cluster;

    #[test]
    fn test_ns_workdir_path_not_exist() {
        let namespace = "test-mynamespace".to_string();
        let name = "test".to_string();
        let cluster_kubeconfig = "/path/kubeconfig/cluster".to_string();
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
            kubeconfig: cluster_kubeconfig,
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
        let cluster_kubeconfig = "/path/kubeconfig/cluster".to_string();
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
            kubeconfig: cluster_kubeconfig,
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
        let cluster_kubeconfig = "/path/kubeconfig/cluster".to_string();
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
            kubeconfig: cluster_kubeconfig,
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
