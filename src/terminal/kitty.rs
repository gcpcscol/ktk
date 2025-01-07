use log::debug;
use palette::{color_difference::Wcag21RelativeContrast, Darken, Srgb};
use std::env;
use std::fmt;
use std::process::{ChildStdout, Command, Stdio};

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

    #[allow(dead_code)]
    pub fn set_tab_color(
        &mut self,
        gradient: colorous::Gradient,
        darken: bool,
        index: usize,
        count: usize,
    ) {
        let col = gradient.eval_rational(index, count);
        self.active_bg = format!("#{:x}", col).to_string();
        let background: Srgb<f32> = Srgb::new(col.r, col.g, col.b).into_format();
        let foreground: Srgb<f32> = Srgb::new(1.0, 1.0, 1.0).into_format();
        if background.has_min_contrast_text(foreground) {
            self.active_fg = "#FFFFFF".to_string()
        } else {
            self.active_fg = "#000000".to_string()
        }
        if !darken {
            self.inactive_fg = format!("#{:x}", col).to_string();
            self.inactive_bg = "NONE".to_string();
            return;
        }
        let inactive_bg_u8: Srgb<u8> =
            Darken::darken(Srgb::new(col.r, col.g, col.b).into_format(), 0.7).into_format::<u8>();
        self.inactive_bg = format!("#{:x}", inactive_bg_u8);
        let background: Srgb<f32> = Srgb::new(
            inactive_bg_u8.red,
            inactive_bg_u8.green,
            inactive_bg_u8.blue,
        )
        .into_format();
        let foreground: Srgb<f32> = Srgb::new(1.0, 1.0, 1.0).into_format();
        if background.has_min_contrast_text(foreground) {
            self.inactive_fg = "#DDDDDD".to_string()
        } else {
            self.inactive_fg = "#222222".to_string()
        }
    }
}

fn kittyls() -> ChildStdout {
    match Command::new("kitty")
        .args(["@", "ls"])
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
            value: serde_json::from_reader(kittyls()).unwrap(),
        }
    }

    pub fn refresh(&mut self) {
        self.value = serde_json::from_reader(kittyls()).unwrap()
    }

    pub fn platform_window_id(&self) -> i64 {
        let mut iow = 0;
        while self.value[iow]["is_focused"].is_boolean() {
            if self.value[iow]["is_focused"].as_bool() == Some(true) {
                let id = self.value[iow]["id"].as_i64().unwrap_or(0);
                return self.value[iow]["platform_window_id"].as_i64().unwrap_or(id);
            }
            iow += 1;
        }
        0
    }

    #[allow(dead_code)]
    pub fn tabs_id(&self) -> Vec<i64> {
        let mut vec = Vec::new();
        let mut iow = 0;
        while self.value[iow].is_object() {
            let mut it = 0;
            while self.value[iow]["tabs"][it].is_object() {
                match self.value[iow]["tabs"][it]["id"].as_i64() {
                    Some(idtab) => vec.push(idtab),
                    None => return vec,
                }
                it += 1;
            }
            iow += 1;
        }
        debug!("tabs_id => {:?}", vec);
        vec
    }

    #[allow(dead_code)]
    pub fn id_path_of_focus_tab(&self) -> Option<IdPath> {
        let mut iow = 0;
        while self.value[iow]["is_focused"].is_boolean() {
            if self.value[iow]["is_focused"].as_bool() == Some(true) {
                let mut it = 0;
                while self.value[iow]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iow]["tabs"][it]["is_focused"].as_bool() == Some(true) {
                        let ret = Some(IdPath {
                            win: self.platform_window_id(),
                            tab: self.value[iow]["tabs"][it]["id"].as_i64().unwrap(),
                        });
                        debug!("id_path_of_focus_tab => {:?}", ret);
                        return ret;
                    }
                    it += 1;
                }
            }
            iow += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn id_of_focus_tab(&self) -> Option<String> {
        let mut iow = 0;
        while self.value[iow]["is_focused"].is_boolean() {
            if self.value[iow]["is_focused"].as_bool() == Some(true) {
                let mut it = 0;
                while self.value[iow]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iow]["tabs"][it]["is_focused"].as_bool() == Some(true) {
                        let ret = Some(self.value[iow]["tabs"][it]["id"].to_string());
                        debug!("id_of_focus_tab => {:?}", ret);
                        return ret;
                    }
                    it += 1;
                }
            }
            iow += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn title_of_focus_tab(&self) -> Option<String> {
        let mut iow = 0;
        while self.value[iow]["is_focused"].is_boolean() {
            if self.value[iow]["is_focused"].as_bool() == Some(true) {
                let mut it = 0;
                while self.value[iow]["tabs"][it]["is_focused"].is_boolean() {
                    if self.value[iow]["tabs"][it]["is_focused"].as_bool() == Some(true) {
                        let ret = Some(self.value[iow]["tabs"][it]["title"].to_string());
                        debug!("title_of_focus_tab => {:?}", ret);
                        return ret;
                    }
                    it += 1;
                }
            }
            iow += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn id_window_with_tab_title(&self, title: &str) -> Option<i64> {
        let mut iow = 0;
        while self.value[iow].is_object() {
            let mut it = 0;
            while self.value[iow]["tabs"][it]["title"].is_string() {
                if self.value[iow]["tabs"][it]["title"].as_str() == Some(title) {
                    let mut iw = 0;
                    while self.value[iow]["tabs"][it]["windows"][iw].is_object() {
                        if self.value[iow]["tabs"][it]["windows"][iw]["is_active_window"]
                            .as_bool()
                            .or(Some(false))
                            .is_some()
                        {
                            let ret = self.value[iow]["tabs"][it]["windows"][iw]["id"]
                                .as_i64()
                                .or(None);
                            debug!("id_window_with_tab_title => {:?}", ret);
                            return ret;
                        }
                        iw += 1;
                    }
                }
                it += 1;
            }
            iow += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn id_tab_with_title(&self, title: &str) -> Option<String> {
        let mut iow = 0;
        while self.value[iow].is_object() {
            let mut it = 0;
            while self.value[iow]["tabs"][it]["title"].is_string() {
                if self.value[iow]["tabs"][it]["title"].as_str() == Some(title) {
                    let ret = self.value[iow]["tabs"][it]["id"].to_string();
                    debug!("id_tab_with_title => {:?}", ret);
                    if ret.is_empty() {
                        return None;
                    };
                    return Some(ret);
                };
                it += 1;
            }
            iow += 1;
        }
        None
    }

    #[allow(dead_code)]
    pub fn tab_title_exist(&self, title: &str) -> bool {
        self.id_tab_with_title(title).is_some()
    }

    #[allow(dead_code)]
    pub fn good_term(&self) -> bool {
        match env::var("TERM") {
            Ok(term) => term == "xterm-kitty",
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn set_tab_title(&self, title: &str) {
        debug!("set_tab_title {}", title);
        Command::new("kitty")
            .arg("@")
            .arg("set-tab-title")
            .arg(title)
            .output()
            .expect("Failed to set tab title");
    }

    pub fn set_tab_color(&self, tab: Tabcolor) {
        debug!("set_tab_color {:?}", tab);
        Command::new("kitty")
            .arg("@")
            .arg("set-tab-color")
            .arg(format!("active_bg={}", tab.active_bg))
            .arg(format!("active_fg={}", tab.active_fg))
            .arg(format!("inactive_bg={}", tab.inactive_bg))
            .arg(format!("inactive_fg={}", tab.inactive_fg))
            .output()
            .expect("Failed to change current tab color");
    }

    #[allow(dead_code)]
    pub fn unset_tab_color(&self) {
        debug!("unset_tab_color");
        let tabc = Tabcolor::new();
        self.set_tab_color(tabc)
    }

    #[allow(dead_code)]
    pub fn set_tab_id_color(&self, idtab: i64, tab: Tabcolor) {
        debug!("set_tab_id_color id:{:?} tab:{:?}", idtab, tab);
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
            .expect("Failed to change tab id:{idtab} color");
    }

    #[allow(dead_code)]
    pub fn unset_tab_id_color(&self, idtab: i64) {
        debug!("unset_tab_id_color");
        let tabc = Tabcolor::new();
        self.set_tab_id_color(idtab, tabc)
    }

    pub fn launch_cmd_in_new_tab_name(&mut self, name: &str, opt: &str, env: &str, cmd: &str) {
        debug!(
            "launch_cmd_in_new_tab_name name:{:?} opt:{:?} env:{:?} cmd:{:?}",
            name, opt, env, cmd
        );
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
            .expect("Failed to launch {cmd} in a new tab");
        self.refresh();
    }

    pub fn launch_shell_in_new_tab_name(&mut self, name: &str) {
        debug!("launch_shell_in_new_tab_name {}", name);
        self.launch_cmd_in_new_tab_name(
            name,
            "",
            "",
            env::var("SHELL")
                .unwrap_or_else(|_| "/usr/bin/bash".to_string())
                .as_str(),
        )
    }

    #[allow(dead_code)]
    pub fn focus_tab_id(&self, id: String) {
        debug!("focus_tab_id {id}");
        Command::new("kitty")
            .arg("@")
            .arg("focus-tab")
            .arg("-m")
            .arg(format!("id:{id}"))
            .output()
            .expect("Failed to focus tab with id:{id}");
    }

    #[allow(dead_code)]
    pub fn focus_execute_tab(&mut self) {
        debug!("focus_execute_tab");
        Command::new("kitty")
            .arg("@")
            .arg("focus-tab")
            .output()
            .expect("Failed to focus tab");
        self.refresh();
    }

    #[allow(dead_code)]
    pub fn focus_window_id(&self, id: i64) {
        debug!("focus_window_id {id}");
        Command::new("kitty")
            .arg("@")
            .arg("focus-window")
            .arg("-m")
            .arg(format!("id:{id}"))
            .output()
            .expect("Failed to focus window with id:{id}");
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
        assert_eq!(k.id_of_focus_tab(), Some("2".to_string()));
    }

    #[test]
    fn test_id_tab_with_title() {
        let k = new_from_file();
        assert_eq!(k.id_tab_with_title("error"), None);
        assert_eq!(k.id_tab_with_title("test"), Some("6".to_string()));
        assert_eq!(k.id_tab_with_title("test2"), Some("7".to_string()));
    }

    #[test]
    fn test_id_window_with_tab_title() {
        let k = new_from_file();
        assert_eq!(k.id_window_with_tab_title("error"), None);
        assert_eq!(k.id_window_with_tab_title("test"), Some(6));
        assert_eq!(k.id_window_with_tab_title("test3"), Some(2));
    }

    #[test]
    fn test_title_of_focus_tab() {
        let k = new_from_file();
        assert_eq!(k.title_of_focus_tab(), Some("\"test3\"".to_string()));
    }

    #[test]
    fn test_platform_window_id() {
        let k = new_from_file();
        assert_eq!(k.platform_window_id(), 20971556);
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

    #[test]
    fn test_tabs_id() {
        let k = new_from_file();
        assert_eq!(k.tabs_id(), vec![1, 6, 7, 2]);
    }
}
