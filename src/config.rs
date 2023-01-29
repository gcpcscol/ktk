use crate::kube::{self, Cluster};
use serde_yaml::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub kubetmp: String,
    pub separator: String,
    pub completion_filename: String,
    pub config_filename: PathBuf,
    pub maxage: u64,
    pub tabprefix: String,
    pub clusters: Vec<Cluster>,
}

impl Context {
    pub fn new(file: &PathBuf) -> Context {
        let f = std::fs::File::open(file).expect("Could not open file.");
        // Deserialize yaml file
        let mut cfg: Value = serde_yaml::from_reader(f).unwrap();
        // Merge anchrors in yaml file
        cfg.apply_merge().unwrap();

        // Populate Context struct
        let value_string = |v: &serde_yaml::Value, def: &str| {
            v.as_str().map_or(def.to_string(), |s| s.to_string())
        };
        let kubetmp = value_string(&cfg["global"]["kubetmp"], "");
        let separator = value_string(&cfg["global"]["separator"], "::");
        let completion_filename =
            value_string(&cfg["global"]["completion"]["file"], "/tmp/tkcomplete");
        let maxage = 86400;
        let tabprefix = value_string(&cfg["global"]["tabprefix"], "");
        let mut i = 0;
        let mut clusts: Vec<Cluster> = Vec::new();
        let value_or_empty =
            |v: &serde_yaml::Value| v.as_str().map_or("".to_string(), |s| s.to_string());

        while cfg["clusters"][i].is_mapping() {
            let name = value_or_empty(&cfg["clusters"][i]["name"]);
            let kcpath = value_string(&cfg["clusters"][i]["kubeconfig"]["path"], "");
            let kcfile = value_string(&cfg["clusters"][i]["kubeconfig"]["file"], "");
            let wdpath = value_string(&cfg["clusters"][i]["workdir"]["path"], "");
            let wdsubdir = value_string(&cfg["clusters"][i]["workdir"]["subdir"], "");
            let prefixns = value_string(&cfg["clusters"][i]["workdir"]["prefixns"], "");
            let tabactivebg = value_string(&cfg["clusters"][i]["kitty"]["tabactivebg"], "NONE");
            let tabinactivebg = value_string(&cfg["clusters"][i]["kitty"]["tabinactivebg"], "NONE");
            let tabactivefg = value_string(&cfg["clusters"][i]["kitty"]["tabactivefg"], "NONE");
            let tabinactivefg = value_string(&cfg["clusters"][i]["kitty"]["tabinactivefg"], "NONE");
            let disabled = cfg["clusters"][i]["disabled"].as_bool().unwrap_or(false);
            let cl: Cluster = Cluster {
                name,
                kubeconfig: format!("{kcpath}/{kcfile}").to_string(),
                workdir: format!("{wdpath}/{wdsubdir}").to_string(),
                prefixns,
                disabled,
                tabcolor: crate::kitty::Tabcolor {
                    active_bg: tabactivebg,
                    inactive_bg: tabinactivebg,
                    active_fg: tabactivefg,
                    inactive_fg: tabinactivefg,
                },
            };
            clusts.push(cl);
            i += 1;
        }
        Context {
            kubetmp,
            separator,
            completion_filename,
            config_filename: (file).to_path_buf(),
            maxage,
            tabprefix,
            clusters: clusts,
        }
    }

    #[allow(dead_code)]
    pub fn clusters_name(&self) -> Vec<String> {
        let mut vec = Vec::new();
        for cl in self.clusters.clone() {
            vec.push(cl.name)
        }
        vec
    }

    pub fn cluster_by_name(&self, search_name: &str) -> Option<&Cluster> {
        self.clusters.iter().find(|&c| c.name == search_name)
    }

    #[allow(dead_code)]
    pub fn completion_file_older_than_config(&self) -> bool {
        let complete_file = self.completion_filename.clone();
        let config_file = self.config_filename.clone();

        let complete_time = match fs::metadata(complete_file).unwrap().modified() {
            Ok(time) => time,
            Err(error) => panic!("Problem opening the file: {error:?}"),
        };

        let config_time = match fs::metadata(config_file).unwrap().modified() {
            Ok(time) => time,
            Err(error) => panic!("Problem opening the file: {error:?}"),
        };

        complete_time < config_time
    }

    pub fn completion_file_older_than_maxage(&self) -> bool {
        let now = SystemTime::now();

        let file = self.completion_filename.clone();
        let maxage = self.maxage;

        if let Ok(time) = fs::metadata(file).unwrap().modified() {
            let diff = now.duration_since(time).unwrap().as_secs();
            return diff > maxage;
        } else {
            println!("Not supported on this platform");
        }
        true
    }

    pub fn update_completion_file(&self) {
        // fetch all namespace in all clusters
        let data_compl = kube::get_all_ns(self.clusters.clone());

        let file = self.completion_filename.clone();
        // Create directory if it don't exist
        let path = Path::new(&file);
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).expect("Could not create destination dir");

        println!("update {file}");
        let str: String = data_compl
            .iter()
            .cloned()
            .map(|x| format!("{x}\n"))
            .collect();

        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(file)
            .expect("Couldn't open file");
        f.write_all(str.as_bytes()).expect("Couldn't write in file");
    }

    pub fn read_completion_file(&self) -> String {
        let result = fs::read(self.completion_filename.clone()).expect("Error in reading the file");
        String::from_utf8(result).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{Cluster, Context};
    use crate::kitty::Tabcolor;
    use crate::PathBuf;
    #[test]
    fn test_new() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path);
        assert_eq!(conf.kubetmp, "/run/user/1000/.kubeconfig");
        assert_eq!(conf.clusters[0].name, "prod");
        assert_eq!(conf.clusters[1].workdir, "/home/user/deploy/deploy_env_dev");
        assert_eq!(conf.clusters[1].tabcolor.active_bg, "#7dcfff");
    }

    #[test]
    fn test_clusters_name() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path);
        assert_eq!(conf.clusters_name(), vec!["prod", "dev", "test"]);
    }

    #[test]
    fn test_cluster_by_name() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path);
        assert_eq!(conf.cluster_by_name("fault"), None);
        assert_ne!(conf.cluster_by_name("prod"), None);
        assert_eq!(
            conf.cluster_by_name("prod"),
            Some(&Cluster {
                name: "prod".to_string(),
                kubeconfig: "/home/user/.kube/konfigs/prod".to_string(),
                workdir: "/home/user/deploy/deploy_env_prod".to_string(),
                prefixns: "".to_string(),
                disabled: false,
                tabcolor: Tabcolor {
                    active_bg: "#db4b4b".to_string(),
                    inactive_bg: "NONE".to_string(),
                    active_fg: "NONE".to_string(),
                    inactive_fg: "#8e3533".to_string()
                }
            })
        );
        let c1 = conf.cluster_by_name("dev").unwrap();
        assert_eq!(c1.workdir, "/home/user/deploy/deploy_env_dev");
    }
}
