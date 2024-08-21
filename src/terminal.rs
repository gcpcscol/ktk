pub mod kitty;
pub mod tmux;
pub mod wezterm;
use std::{env, process};

use log::{debug, error};

pub struct Kitty {
    context: kitty::Context,
}

pub struct Tmux {
    context: tmux::Context,
}

pub struct WezTerm {
    context: wezterm::Context,
}

#[allow(dead_code)]
pub trait Terminal {
    fn good_term(&self) -> bool;
    fn identifier(&self) -> String;
    fn id_of_focus_tab(&self) -> Option<String>;
    fn id_path_of_focus_tab(&self) -> Option<String>;
    fn focus_tab_name(&self, name: &str) -> bool;
    fn create_new_tab(&mut self, name: &str);
    fn change_tab_title(&self, name: &str);
    fn change_tab_color(&self, color: kitty::Tabcolor);
}

pub fn detect() -> Box<dyn Terminal> {
    let other = "other".to_string();
    match env::var("TERM_PROGRAM").unwrap_or(other.clone()).as_str() {
        "tmux" => {
            debug!("Tmux terminal");
            Box::new(Tmux {
                context: tmux::Context::new(),
            })
        }
        "WezTerm" => {
            debug!("WezTerm terminal");
            Box::new(WezTerm {
                context: wezterm::Context::new(),
            })
        }
        _ => {
            if env::var("TERMINAL").unwrap_or(other) == "kitty" {
                debug!("Kitty terminal");
                Box::new(Kitty {
                    context: kitty::Context::new(),
                })
            } else {
                error!("Only supports Kitty, WezTerm and Tmux for now.");
                process::exit(42)
            }
        }
    }
}

impl Terminal for Kitty {
    fn good_term(&self) -> bool {
        self.context.good_term()
    }

    fn identifier(&self) -> String {
        format!("kitty-{}", self.context.platform_window_id())
    }

    fn id_of_focus_tab(&self) -> Option<String> {
        self.context.id_of_focus_tab()
    }

    fn id_path_of_focus_tab(&self) -> Option<String> {
        self.context
            .id_path_of_focus_tab()
            .map(|expr| format!("kitty-{}", expr))
    }

    fn focus_tab_name(&self, name: &str) -> bool {
        if let Some(idwin) = self.context.id_window_with_tab_title(name) {
            self.context.focus_window_id(idwin);
            return true;
        }
        false
    }

    fn create_new_tab(&mut self, name: &str) {
        self.context.launch_shell_in_new_tab_name(name);
    }

    fn change_tab_title(&self, name: &str) {
        self.context.set_tab_title(name);
    }

    fn change_tab_color(&self, color: kitty::Tabcolor) {
        self.context.set_tab_color(color);
    }
}

impl Terminal for Tmux {
    fn good_term(&self) -> bool {
        self.context.good_term()
    }

    fn identifier(&self) -> String {
        format!("tmux-{}", self.context.current_session())
    }

    fn id_of_focus_tab(&self) -> Option<String> {
        self.context.id_of_current_window()
    }

    fn id_path_of_focus_tab(&self) -> Option<String> {
        self.context
            .id_path_of_current_window()
            .map(|expr| format!("tmux-{}", expr.trim_end().to_string()))
    }

    fn focus_tab_name(&self, name: &str) -> bool {
        self.context.select_window_name(name)
    }

    fn create_new_tab(&mut self, name: &str) {
        self.context.launch_shell_in_new_tab_name(name);
    }

    fn change_tab_title(&self, name: &str) {
        self.context.set_tab_title(name);
    }

    fn change_tab_color(&self, _: kitty::Tabcolor) {}
}

impl Terminal for WezTerm {
    fn good_term(&self) -> bool {
        self.context.good_term()
    }

    fn identifier(&self) -> String {
        format!("wezterm-{}", self.context.platform_window_id())
    }

    fn id_of_focus_tab(&self) -> Option<String> {
        self.context.id_of_focus_tab()
    }

    fn id_path_of_focus_tab(&self) -> Option<String> {
        self.context
            .id_path_of_focus_tab()
            .map(|expr| format!("wezterm-{}", expr))
    }

    fn focus_tab_name(&self, name: &str) -> bool {
        if let Some(id) = self.context.id_tab_with_title(name) {
            self.context.focus_tab_id(id);
            return true;
        }
        false
    }

    fn create_new_tab(&mut self, name: &str) {
        let _ = self.context.launch_shell_in_new_tab_name(name);
    }

    fn change_tab_title(&self, name: &str) {
        self.context.set_tab_title(name);
    }

    fn change_tab_color(&self, _: kitty::Tabcolor) {}
}
