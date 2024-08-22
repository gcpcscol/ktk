use log::debug;
use std::env;
use std::fmt;
use std::process::{ChildStdout, Command, Stdio};

#[derive(Debug)]
pub struct Context {
    value: serde_json::Value,
    client: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdPath {
    pub win: i64,
    pub tab: i64,
}

impl fmt::Display for IdPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.win, self.tab)
    }
}

fn weztermls() -> ChildStdout {
    match Command::new("wezterm")
        .args(["cli", "list", "--format=json"])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(v) => v.stdout.unwrap(),
        Err(e) => {
            panic!("Error {e:?}");
        }
    }
}

fn weztermlsclient() -> ChildStdout {
    match Command::new("wezterm")
        .args(["cli", "list-clients", "--format=json"])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(v) => v.stdout.unwrap(),
        Err(e) => {
            panic!("Error {e:?}");
        }
    }
}

impl Context {
    pub fn new() -> Context {
        Context {
            value: serde_json::from_reader(weztermls()).unwrap(),
            client: serde_json::from_reader(weztermlsclient()).unwrap(),
        }
    }

    pub fn refresh(&mut self) {
        self.value = serde_json::from_reader(weztermls()).unwrap();
        self.client = serde_json::from_reader(weztermlsclient()).unwrap();
    }

    pub fn platform_window_id(&self) -> i64 {
        0
    }

    #[allow(dead_code)]
    pub fn tabs_id(&self) -> Vec<i64> {
        let mut vec = Vec::new();
        let mut it = 0;
        while self.value[it].is_object() {
            match self.value[it]["tab_id"].as_i64() {
                Some(idtab) => {
                    if !vec.contains(&idtab) {
                        vec.push(idtab);
                    }
                }
                None => return vec,
            }
            it += 1;
        }
        debug!("tabs_id => {:?}", vec);
        vec
    }

    #[allow(dead_code)]
    pub fn id_of_focus_tab(&self) -> Option<String> {
        let pane_id = self.client[0]["focused_pane_id"].as_i64();
        debug!("id_of_focus_pane => {:?}", pane_id);
        let mut it = 0;
        while self.value[it].is_object() {
            if self.value[it]["pane_id"].as_i64().or(None) == pane_id {
                return Some(self.value[it]["tab_id"].to_string());
            }
            it += 1;
        }
        return None;
    }

    pub fn id_path_of_focus_tab(&self) -> Option<IdPath> {
        let win_id = self.platform_window_id();
        let pane_id = self.client[0]["focused_pane_id"].as_i64();
        debug!("id_of_focus_pane => {:?}", pane_id);
        let mut it = 0;
        while self.value[it].is_object() {
            if self.value[it]["pane_id"].as_i64().or(None) == pane_id {
                return Some(IdPath {
                    win: win_id,
                    tab: self.value[it]["tab_id"].as_i64().unwrap(),
                });
            }
            it += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn id_of_focus_pane(&self) -> Option<String> {
        let ret = Some(self.client[0]["focused_pane_id"].to_string());
        debug!("id_of_focus_pane => {:?}", ret);
        return ret;
    }

    #[allow(dead_code)]
    pub fn title_of_focus_tab(&self) -> Option<String> {
        let pane_id = self.client[0]["focused_pane_id"].as_i64();
        debug!("id_of_focus_pane => {:?}", pane_id);
        let mut it = 0;
        while self.value[it].is_object() {
            if self.value[it]["pane_id"].as_i64().or(None) == pane_id {
                debug!("id_of_focus_tab => {:?}", self.value[it]["tab_id"]);
                return Some(self.value[it]["tab_title"].to_string());
            }
            it += 1;
        }
        return None;
    }

    #[allow(dead_code)]
    pub fn id_tab_with_title(&self, title: &str) -> Option<String> {
        let mut it = 0;
        while self.value[it]["tab_title"].is_string() {
            if self.value[it]["tab_title"].as_str() == Some(title) {
                let ret = self.value[it]["tab_id"].to_string();
                debug!("id_tab_with_title => {:?}", ret);
                if ret == "" {
                    return None;
                }
                return Some(ret);
            };
            it += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn tab_title_exist(&self, title: &str) -> bool {
        self.id_tab_with_title(title).is_some()
    }

    #[allow(dead_code)]
    pub fn good_term(&self) -> bool {
        match env::var("TERM_PROGRAM") {
            Ok(term) => term == "WezTerm",
            Err(_) => false,
        }
    }

    pub fn set_tab_title(&self, title: &str) {
        debug!("set_tab_title => {}", title);
        Command::new("wezterm")
            .arg("cli")
            .arg("set-tab-title")
            .arg(title)
            .output()
            .expect("Failed to set tab title");
    }

    #[allow(dead_code)]
    pub fn set_tab_title_for_pane_id(&self, title: &str, pane_id: &str) {
        debug!("set_tab_title {} for pane_id {}", title, pane_id);
        Command::new("wezterm")
            .arg("cli")
            .arg("set-tab-title")
            .arg(title)
            .arg(format!("--pane-id={}", pane_id))
            .output()
            .expect("Failed to set tab title");
    }

    pub fn launch_cmd_in_new_tab_name(&mut self, name: &str, opt: &str, env: &str, cmd: &str) {
        debug!(
            "launch_cmd_in_new_tab_name name:{:?} opt:{:?} env:{:?} cmd:{:?}",
            name, opt, env, cmd
        );
        let output = Command::new("wezterm")
            .arg("cli")
            .arg("spawn")
            .arg("--")
            .arg(cmd)
            .output()
            .expect("failed");
        let pane_id = format!(
            "{}",
            String::from_utf8_lossy(&output.stdout)
                .to_string()
                .trim_end()
        );
        let opt = format!("--pane-id={}", pane_id);
        debug!("Execute => wezterm cli set-tab-title '{name}' {opt}");
        Command::new("wezterm")
            .arg("cli")
            .arg("set-tab-title")
            .arg(name)
            .arg(opt)
            .output()
            .expect("Failed to set tab title");
        // self.refresh();
        // if let Some(id) = self.id_tab_with_title(name) {
        //     self.focus_tab_id(id);
        // }
        self.refresh();
    }

    pub fn launch_shell_in_new_tab_name(&mut self, name: &str) {
        debug!("launch_shell_in_new_tab_name => {}", name);
        self.launch_cmd_in_new_tab_name(
            name,
            "",
            "",
            env::var("SHELL")
                .unwrap_or_else(|_| "/usr/bin/bash".to_string())
                .as_str(),
        );
    }

    #[allow(dead_code)]
    pub fn focus_tab_id(&self, id: String) {
        debug!("focus_tab_id => {id}");
        Command::new("wezterm")
            .arg("cli")
            .arg("activate-tab")
            .arg(format!("--tab-id={id}"))
            .output()
            .expect("Failed to focus tab with id:{id}");
    }

    #[allow(dead_code)]
    pub fn focus_pane_id(&self, id: i64) {
        debug!("focus_pane_id => {id}");
        Command::new("wezterm")
            .arg("cli")
            .arg("active-pane")
            .arg(format!("--pane-id={id}"))
            .output()
            .expect("Failed to focus pane with id:{id}");
    }
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::io::*;
    use crate::PathBuf;
    use std::fs::File;

    fn new_from_file() -> Context {
        let pathls = PathBuf::from("./tests/wezterm-list.json");
        let filels = File::open(pathls).expect("Failed to open file");
        let weztermout = BufReader::new(filels);
        let pathcli = PathBuf::from("./tests/wezterm-list-clients.json");
        let filecli = File::open(pathcli).expect("Failed to open file");
        let weztermcliout = BufReader::new(filecli);
        Context {
            value: serde_json::from_reader(weztermout).unwrap(),
            client: serde_json::from_reader(weztermcliout).unwrap(),
        }
    }

    #[test]
    fn test_id_of_focus_pane() {
        let k = new_from_file();
        assert_eq!(k.id_of_focus_pane(), Some("10".to_string()));
    }

    #[test]
    fn test_id_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(k.id_of_focus_tab(), Some("8".to_string()));
    }

    #[test]
    fn test_id_tab_with_title() {
        let k = new_from_file();
        assert_eq!(k.id_tab_with_title("error"), None);
        assert_eq!(k.id_tab_with_title("test"), Some("4".to_string()));
        assert_eq!(k.id_tab_with_title("test2"), Some("7".to_string()));
    }

    #[test]
    fn test_title_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(k.title_of_focus_tab(), Some("\"test3\"".to_string()));
    }

    #[test]
    fn test_platform_window_id() {
        let k = new_from_file();
        assert_eq!(k.platform_window_id(), 0);
    }

    #[test]
    fn test_tabs_id() {
        let k = new_from_file();
        assert_eq!(k.tabs_id(), vec![0, 4, 6, 7, 8, 9, 10, 11]);
    }
}
