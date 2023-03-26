use std::env;
use std::process::Command;

#[derive(Debug)]
pub struct Context {}

impl Context {
    #[allow(dead_code)]
    pub fn new() -> Context {
        Context {}
    }

    #[allow(dead_code)]
    pub fn good_term(&self) -> bool {
        match env::var("TERM") {
            Ok(term) => term.contains("tmux"),
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn current_session(&self) -> String {
        match Command::new("tmux")
            .arg("display-message")
            .arg("-p")
            .arg("#S")
            .output()
            .map(|x| x.stdout)
        {
            Ok(s) => String::from_utf8(s).unwrap().trim_end().to_string(),
            Err(_) => std::process::exit(1),
        }
    }

    #[allow(dead_code)]
    pub fn id_of_current_window(&self) -> Option<String> {
        match Command::new("tmux")
            .arg("display-message")
            .arg("-p")
            .arg("#{window_id}")
            .output()
            .map(|x| x.stdout)
        {
            Ok(s) => Some(String::from_utf8(s).unwrap().trim_end().to_string()),
            Err(_) => std::process::exit(1),
        }
    }

    #[allow(dead_code)]
    pub fn id_path_of_current_window(&self) -> Option<String> {
        match Command::new("tmux")
            .arg("list-windows")
            .arg("-F")
            .arg("#{session_name}/#{window_id}")
            .arg("-f")
            .arg("#{m:\\*,#{window_flags}}")
            .output()
            .map(|x| x.stdout)
        {
            Ok(s) => Some(String::from_utf8(s).unwrap()),
            Err(e) => panic!("id_path_of {}", e),
        }
    }

    pub fn id_of_window_name(&self, name: &str) -> Option<String> {
        match Command::new("tmux")
            .arg("list-windows")
            .arg("-F")
            .arg("#{window_id}")
            .arg("-f")
            .arg(format!("#{{m:{name},#{{window_name}}}}"))
            .output()
            .map(|x| x.stdout)
        {
            Ok(s) => {
                let v = String::from_utf8(s).unwrap();
                return Some(String::from(v.trim_end()));
            }
            Err(_) => None,
        }
    }

    pub fn select_window_name(&self, name: &str) -> bool {
        match self.id_of_window_name(name) {
            Some(idwin) => {
                Command::new("tmux")
                    .arg("select-window")
                    .arg("-t")
                    .arg(idwin)
                    .output()
                    .expect("Failed to select tmux window");
                return true;
            }
            None => false,
        }
    }

    pub fn launch_cmd_in_new_tab_name(&self, name: &str, dir: &str, env: &str, cmd: &str) {
        Command::new("tmux")
            .arg("new-window")
            .arg("-n")
            .arg(name)
            .arg("-e")
            .arg(env)
            .arg("-c")
            .arg(dir)
            .arg(cmd)
            .output()
            .expect("Failed to launch tmux window");
    }

    pub fn launch_shell_in_new_tab_name(&self, name: &str) {
        self.launch_cmd_in_new_tab_name(
            name,
            "",
            "",
            env::var("SHELL")
                .unwrap_or_else(|_| "/usr/bin/bash".to_string())
                .as_str(),
        )
    }

    pub fn set_tab_title(&self, name: &str) {
        match self.id_of_window_name(name) {
            Some(idwin) => {
                Command::new("tmux")
                    .arg("rename-window")
                    .arg("-t")
                    .arg(idwin)
                    .arg(name)
                    .output()
                    .expect("Failed to launch tmux window");
            }
            None => println!("Error set_tab_title"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Context;

    #[test]
    fn get_current_session() {
        let t = Context::new();
        assert_eq!(t.current_session(), "0");
    }
}
