//! Read ktk yaml file and load Context
use crate::kube::{self, Cluster};
use clap::crate_name;
use palette::Darken;
use serde_yaml::Value;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::time::SystemTime;

use log::{error, info};
use owo_colors::OwoColorize;
use palette::{color_difference::Wcag21RelativeContrast, Srgb};

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

pub fn select_contrasting_fg_color(hexcolor: &str, active: bool) -> String {
    let col = csscolorparser::parse(hexcolor).unwrap_or_default();
    if active {
        let fg = csscolorparser::parse("#FFFFFF").unwrap();
        let fg_active: Srgb<f32> = Srgb::new(fg.r, fg.g, fg.b).into_format();
        let background: Srgb<f32> = Srgb::new(col.r, col.g, col.b).into_format();
        if background.has_min_contrast_large_text(fg_active) {
            return "#FFFFFF".to_string();
        } else {
            return "#000000".to_string();
        }
    } else {
        let fg = csscolorparser::parse("#DDDDDD").unwrap();
        let fg_active: Srgb<f32> = Srgb::new(fg.r, fg.g, fg.b).into_format();
        let background: Srgb<f32> = Srgb::new(col.r, col.g, col.b).into_format();
        if background.has_min_contrast_large_text(fg_active) {
            return "#DDDDDD".to_string();
        } else {
            return "#222222".to_string();
        }
    }
}

impl Context {
    pub fn new(file: &PathBuf, notimeout: bool) -> Context {
        //! Load config file in struct Context
        let f = std::fs::File::open(file).expect("Could not open file.");
        // Deserialize yaml file
        let mut cfg: Value = match serde_yaml::from_reader(f) {
            Ok(v) => v,
            Err(e) => {
                error!("Unabled to load config file {} : {e}", file.display());
                process::exit(52)
            }
        };
        // Merge anchrors in yaml file
        cfg.apply_merge().unwrap();

        // Populate Context struct
        let value_string = |v: &serde_yaml::Value, def: &str| {
            v.as_str().map_or(def.to_string(), |s| s.to_string())
        };
        let kubetmp = value_string(
            &cfg["global"]["kubetmp"],
            format!("/tmp/{}", crate_name!()).as_str(),
        );

        let pathktmp = Path::new(&kubetmp);
        let parentktmp = pathktmp.parent().unwrap();
        if fs::create_dir_all(parentktmp).is_err() {
            error!("Could not create destination dir for kubetmp {kubetmp}");
            process::exit(53)
        }

        let separator = value_string(&cfg["global"]["separator"], "::");
        let completion_filename =
            value_string(&cfg["global"]["completion"]["file"], "/tmp/tkcomplete");

        let pathcf = Path::new(&completion_filename);
        let parentcf = pathcf.parent().unwrap();
        if fs::create_dir_all(parentcf).is_err() {
            error!(
                "Could not create destination dir for completion_filename {completion_filename}"
            );
            process::exit(53)
        }

        let maxage = cfg["global"]["completion"]["maxage"]
            .as_u64()
            .unwrap_or(3600);
        let tabprefix = value_string(&cfg["global"]["tabprefix"], "");
        let mut i = 0;
        let mut clusters: Vec<Cluster> = Vec::new();
        let value_or_empty =
            |v: &serde_yaml::Value| v.as_str().map_or("".to_string(), |s| s.to_string());

        while cfg["clusters"][i].is_mapping() {
            let name = value_or_empty(&cfg["clusters"][i]["name"]);
            let kubeconfig_path = format!(
                "{}/{}",
                value_string(&cfg["clusters"][i]["kubeconfig"]["path"], ""),
                value_string(&cfg["clusters"][i]["kubeconfig"]["file"], "")
            );
            let workdir = format!(
                "{}/{}",
                value_string(&cfg["clusters"][i]["workdir"]["path"], ""),
                value_string(&cfg["clusters"][i]["workdir"]["subdir"], "")
            );
            let prefixns = value_string(&cfg["clusters"][i]["workdir"]["prefixns"], "");
            let active_bg = value_string(&cfg["clusters"][i]["kitty"]["tabactivebg"], "NONE");
            //let inactive_bg = value_string(&cfg["clusters"][i]["kitty"]["tabinactivebg"], "NONE");
            let bg = csscolorparser::parse(active_bg.as_str())
                .unwrap_or_default()
                .to_linear_rgba();
            let inac_bg: Srgb<u8> =
                Darken::darken(Srgb::new(bg.0, bg.1, bg.2), 0.01).into_format::<u8>();
            let inactive_bg = format!("{:x}", inac_bg);
            let active_fg = select_contrasting_fg_color(&active_bg, true);
            let inactive_fg = select_contrasting_fg_color(&inactive_bg, false);
            let disabled = cfg["clusters"][i]["disabled"].as_bool().unwrap_or(false);
            let timeout = cfg["clusters"][i]["kubeconfig"]["timeout"]
                .as_u64()
                .map_or(10, |t| if notimeout { 60 } else { t });
            let cl: Cluster = Cluster {
                name,
                kubeconfig_path,
                workdir,
                prefixns,
                disabled,
                timeout: timeout.try_into().unwrap_or(10),
                tabcolor: crate::terminal::kitty::Tabcolor {
                    active_bg,
                    inactive_bg,
                    active_fg,
                    inactive_fg,
                },
            };
            clusters.push(cl);
            i += 1;
        }
        Context {
            kubetmp,
            separator,
            completion_filename,
            config_filename: (file).to_path_buf(),
            maxage,
            tabprefix,
            clusters,
        }
    }

    #[allow(dead_code)]
    pub fn clusters_names(&self) -> Vec<String> {
        // returns the list of cluster names
        let mut vec = Vec::new();
        for cl in self.clusters.clone() {
            if !cl.disabled {
                vec.push(cl.name)
            }
        }
        vec
    }

    #[allow(dead_code)]
    pub fn cluster_named(&self, search_name: &str) -> Option<&Cluster> {
        // returns the cluster with name search_name
        self.clusters.iter().find(|&c| c.name == search_name)
    }

    #[allow(dead_code)]
    pub fn nb_clusters(&self) -> (u32, u32) {
        let mut active = 0;
        let mut inactive = 0;
        for cl in self.clusters.iter() {
            if cl.disabled {
                inactive += 1;
            } else {
                active += 1
            }
        }
        return (active, inactive);
    }

    #[allow(dead_code)]
    pub fn list_clusters_by_state(&self, active: bool) {
        let clusters = self.clusters.clone();
        let mut i = 0;
        for cl in clusters.iter() {
            if cl.disabled != active {
                i += 1;
                let bg = csscolorparser::parse(cl.tabcolor.active_bg.as_str())
                    .unwrap_or_default()
                    .to_linear_rgba_u8();
                let fg = csscolorparser::parse(cl.tabcolor.active_fg.as_str())
                    .unwrap_or_default()
                    .to_linear_rgba_u8();
                let inbg = csscolorparser::parse(cl.tabcolor.inactive_bg.as_str())
                    .unwrap_or_default()
                    .to_linear_rgba_u8();
                let infg = csscolorparser::parse(cl.tabcolor.inactive_fg.as_str())
                    .unwrap_or_default()
                    .to_linear_rgba_u8();
                println!(
                    "{i:>4} - {} -> inactive tab: {}",
                    cl.name
                        .on_truecolor(bg.0, bg.1, bg.2)
                        .truecolor(fg.0, fg.1, fg.2),
                    cl.name
                        .on_truecolor(inbg.0, inbg.1, inbg.2)
                        .truecolor(infg.0, infg.1, infg.2)
                        .italic()
                )
            }
        }
    }

    #[allow(dead_code)]
    pub fn list_clusters(&self) {
        // displays the list of clusters with their colour
        let (nbactive, nbinactive) = self.nb_clusters();
        if nbactive > 0 {
            println!("List of active clusters:");
            self.list_clusters_by_state(true);
        }

        if nbinactive > 0 {
            println!("List of inactive clusters:");
            self.list_clusters_by_state(false);
        }
    }

    #[allow(dead_code)]
    pub fn completion_file_older_than_config(&self) -> bool {
        if !Path::new(&self.completion_filename).exists() {
            return true;
        }

        let complete_file = self.completion_filename.clone();
        let config_file = self.config_filename.clone();

        let complete_time = match fs::metadata(complete_file).unwrap().modified() {
            Ok(time) => time,
            Err(error) => {
                error!("Problem opening the file: {error:?}");
                process::exit(10)
            }
        };

        let config_time = match fs::metadata(config_file).unwrap().modified() {
            Ok(time) => time,
            Err(error) => {
                error!("Problem opening the file: {error:?}");
                process::exit(10)
            }
        };

        complete_time < config_time
    }

    pub fn completion_file_older_than_maxage(&self) -> bool {
        if !Path::new(&self.completion_filename).exists() {
            return true;
        }

        let now = SystemTime::now();

        let file = self.completion_filename.clone();
        let maxage = self.maxage;

        if let Ok(time) = fs::metadata(file).unwrap().modified() {
            let diff = now.duration_since(time).unwrap().as_secs();
            return diff > maxage;
        } else {
            error!("Not supported on this platform");
        }
        true
    }

    pub fn update_completion_file(&self) {
        // fetch all namespace in all clusters
        let data_compl = kube::get_all_ns(self.clusters.clone(), self.separator.clone());

        // Do not change the completion file if no cluster can be reached.
        if data_compl.is_empty() {
            error!("no cluster is reachable");
            return;
        }
        let file = self.completion_filename.clone();
        // Create directory if it don't exist
        let path = Path::new(&file);
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).expect("Could not create destination dir");

        info!("update {file}");
        let str: String = data_compl
            .iter()
            .cloned()
            .map(|x| if !x.is_empty() { format!("{x}\n") } else { x })
            .collect();

        let mut f = File::create(file.clone()).expect("Couldn't open file");

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
    use crate::terminal::kitty::Tabcolor;
    use crate::PathBuf;
    #[test]
    fn test_new() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path, false);
        assert_eq!(conf.kubetmp, "/run/user/1000/.kubeconfig");
        assert_eq!(conf.maxage, 86400);
        assert_eq!(conf.clusters[0].name, "prod");
        assert_eq!(conf.clusters[1].workdir, "~/deploy/deploy_env_dev");
        assert_eq!(conf.clusters[1].tabcolor.active_bg, "#7dcfff");
    }

    #[test]
    fn test_clusters_name() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path, false);
        assert_eq!(conf.clusters_names(), vec!["prod", "dev"]);
    }

    #[test]
    fn test_cluster_by_name() {
        let path = PathBuf::from("./conf/config.sample.yaml");
        let conf = Context::new(&path, false);
        assert_eq!(conf.cluster_named("fault"), None);
        assert_ne!(conf.cluster_named("prod"), None);
        assert_eq!(
            conf.cluster_named("prod"),
            Some(&Cluster {
                name: "prod".to_string(),
                kubeconfig_path: "~/.kube/konfigs/prod".to_string(),
                workdir: "~/deploy/deploy_env_prod".to_string(),
                prefixns: "".to_string(),
                disabled: false,
                timeout: 5,
                tabcolor: Tabcolor {
                    active_bg: "#db4b4b".to_string(),
                    inactive_bg: "NONE".to_string(),
                    active_fg: "NONE".to_string(),
                    inactive_fg: "#8e3533".to_string()
                }
            })
        );
        let c1 = conf.cluster_named("dev").unwrap();
        assert_eq!(c1.workdir, "~/deploy/deploy_env_dev");
    }
}
