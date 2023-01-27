use std::env;
use std::fmt;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct Context {
    value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tabcolor {
    pub active_bg: String,
    pub inactive_bg: String,
    pub active_fg: String,
    pub inactive_fg: String,
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

impl Tabcolor {
    #[allow(dead_code)]
    pub fn new() -> Tabcolor {
        Tabcolor {
            active_bg: "NONE".to_string(),
            inactive_bg: "NONE".to_string(),
            active_fg: "NONE".to_string(),
            inactive_fg: "NONE".to_string(),
        }
    }
}

impl Context {
    pub fn new() -> Context {
        let kittyout = Command::new("kitty")
            .args(["@", "ls"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to load kitty");

        Context {
            value: serde_json::from_reader(kittyout.stdout.expect("Failed")).unwrap(),
        }
    }

    pub fn refresh(&mut self) {
        let kittyout = Command::new("kitty")
            .args(["@", "ls"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to load kitty");

        self.value = serde_json::from_reader(kittyout.stdout.expect("Failed")).unwrap();
    }

    pub fn platform_window_id(&self) -> Option<i64> {
        let mut iw = 0;
        while self.value[iw]["is_focused"].is_boolean() {
            if self.value[iw]["is_focused"].as_bool().expect("Error") {
                return self.value[iw]["platform_window_id"].as_i64().or(None);
            }
            iw += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn tabs_id(&self) -> Vec<i64> {
        let mut it = 0;
        let mut vec = Vec::new();
        while self.value[0]["tabs"][it]["id"].is_i64() {
            vec.push(self.value[0]["tabs"][it]["id"].as_i64().expect("Error"));
            it += 1;
        }
        vec
    }

    #[allow(dead_code)]
    pub fn id_path_of_focus_tab(&self) -> Option<IdPath> {
        let mut it = 0;
        let mut iw = 0;
        while self.value[iw]["is_focused"].is_boolean() {
            if self.value[iw]["is_focused"].as_bool().expect("Error") {
                while self.value[iw]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iw]["tabs"][it]["is_focused"]
                        .as_bool()
                        .expect("Error")
                    {
                        let idpath: IdPath = IdPath {
                            win: self.value[iw]["platform_window_id"]
                                .as_i64()
                                .expect("Failed to find kitty platform window id"),
                            tab: self.value[iw]["tabs"][it]["id"].as_i64().expect("Error"),
                        };
                        return Some(idpath);
                    };
                    it += 1;
                }
            }
            iw += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn id_of_focus_tab(&self) -> Option<i64> {
        let mut it = 0;
        let mut iw = 0;
        while self.value[iw]["is_focused"].is_boolean() {
            if self.value[iw]["is_focused"].as_bool().expect("Error") {
                while self.value[iw]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iw]["tabs"][it]["is_focused"]
                        .as_bool()
                        .expect("Error")
                    {
                        return self.value[iw]["tabs"][it]["id"].as_i64().or(None);
                    };
                    it += 1;
                }
            }
            iw += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn title_of_focus_tab(&self) -> Option<String> {
        let mut it = 0;
        let mut iw = 0;
        while self.value[iw]["is_focused"].is_boolean() {
            if self.value[iw]["is_focused"].as_bool().expect("Error") {
                while self.value[iw]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iw]["tabs"][it]["is_focused"]
                        .as_bool()
                        .expect("Error")
                    {
                        return Some(self.value[iw]["tabs"][it]["title"].to_string());
                    };
                    it += 1;
                }
            }
            iw += 1;
        }
        None
    }

    pub fn id_tab_with_title(&self, title: &str) -> Option<i64> {
        let mut i = 0;
        let mut iw = 0;
        while self.value[iw]["is_focused"].is_boolean() {
            if self.value[iw]["is_focused"].as_bool().expect("Error") {
                while self.value[iw]["tabs"][i]["title"].is_string() {
                    if self.value[iw]["tabs"][i]["title"].as_str().expect("Error") == title {
                        return self.value[iw]["tabs"][i]["id"].as_i64().or(None);
                    };
                    i += 1;
                }
            }
            iw += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn tab_title_exist(&self, title: &str) -> bool {
        let id = self.id_tab_with_title(title);
        id.is_some()
    }

    #[allow(dead_code)]
    pub fn good_term(&self) -> bool {
        let shell = env::var("TERM").unwrap_or_else(|_| "?".to_string());
        shell == "xterm-kitty"
    }

    #[allow(dead_code)]
    pub fn set_tab_title(&self, title: &str) {
        Command::new("kitty")
            .arg("@")
            .arg("set-tab-title")
            .arg(title)
            .output()
            .expect("Failed to load kitty");
    }

    pub fn set_tab_color(&self, tab: Tabcolor) {
        Command::new("kitty")
            .arg("@")
            .arg("set-tab-color")
            .arg(format!("active_bg={}", tab.active_bg))
            .arg(format!("active_fg={}", tab.active_fg))
            .arg(format!("inactive_bg={}", tab.inactive_bg))
            .arg(format!("inactive_fg={}", tab.inactive_fg))
            .output()
            .expect("Failed to load kitty");
    }

    #[allow(dead_code)]
    pub fn unset_tab_color(&self) {
        let tabc = Tabcolor::new();
        self.set_tab_color(tabc)
    }

    #[allow(dead_code)]
    pub fn set_tab_id_color(&self, idtab: i64, tab: Tabcolor) {
        Command::new("kitty")
            .arg("@")
            .arg("set-tab-color")
            .arg("-m")
            .arg(format!("\"id:{idtab}\""))
            .arg(format!("active_bg={}", tab.active_bg))
            .arg(format!("active_fg={}", tab.active_fg))
            .arg(format!("inactive_bg={}", tab.inactive_bg))
            .arg(format!("inactive_fg={}", tab.inactive_fg))
            .output()
            .expect("Failed to load kitty");
    }

    #[allow(dead_code)]
    pub fn unset_tab_id_color(&self, idtab: i64) {
        let tabc = Tabcolor::new();
        self.set_tab_id_color(idtab, tabc)
    }

    pub fn launch_cmd_in_new_tab_name(&mut self, name: &str, opt: &str, env: &str, cmd: &str) {
        Command::new("kitty")
            .arg("@")
            .arg("launch")
            .arg("--type=tab")
            .arg("--tab-title")
            .arg(name)
            .arg(opt)
            .arg("--env")
            .arg(env)
            .arg(cmd)
            .output()
            .expect("Failed to load kitty");
        self.refresh();
    }

    pub fn launch_shell_in_new_tab_name(&mut self, name: &str) {
        self.launch_cmd_in_new_tab_name(
            name,
            "",
            "",
            env::var("SHELL")
                .unwrap_or_else(|_| "/usr/bin/bash".to_string())
                .as_str(),
        )
    }

    pub fn focus_tab_id(&self, id: i64) {
        Command::new("kitty")
            .arg("@")
            .arg("focus-tab")
            .arg("-m")
            .arg(format!("id:{id}"))
            .output()
            .expect("Failed to load kitty");
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, IdPath};
    use crate::io::*;
    use crate::PathBuf;
    use std::fs::File;

    fn new_from_file() -> Context {
        let path = PathBuf::from("./tests/kitty.json");
        let file = File::open(path).expect("Failed to open file");
        let kittyout = BufReader::new(file);
        Context {
            value: serde_json::from_reader(kittyout).unwrap(),
        }
    }

    #[test]
    fn test_id_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(k.id_of_focus_tab(), Some(2));
    }

    #[test]
    fn test_id_tab_with_title() {
        let k = new_from_file();
        assert_eq!(k.id_tab_with_title("error"), None);
        assert_eq!(k.id_tab_with_title("test2"), Some(7));
    }

    #[test]
    fn test_title_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(
            k.title_of_focus_tab(),
            Some("\"kitty @ ls > kitty.json\"".to_string())
        );
    }

    #[test]
    fn test_platform_window_id() {
        let k = new_from_file();
        assert_eq!(k.platform_window_id(), Some(20971556));
    }

    #[test]
    fn test_id_path_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(
            k.id_path_of_focus_tab(),
            Some(IdPath {
                win: 20971556,
                tab: 2
            })
        );
    }
}
