pub mod kitty;
use std::{env, process};

pub struct Kitty {
    context: kitty::Context,
}

pub trait Terminal {
    fn identifier(&self) -> String;
    fn id_of_focus_tab(&self) -> Option<i64>;
    fn id_path_of_focus_tab(&self) -> Option<String>;
    fn focus_tab_name(&self, name: &str) -> bool;
    fn create_new_tab(&mut self, name: &str);
    fn change_tab_title(&self, name: &str);
    fn change_tab_color(&self, color: kitty::Tabcolor);
}

pub fn detect() -> Box<dyn Terminal> {
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return Box::new(Kitty {
            context: kitty::Context::new(),
        });
    } else {
        process::exit(42)
    }
}

impl Terminal for Kitty {
    fn identifier(&self) -> String {
        format!("{}", self.context.platform_window_id())
    }

    fn id_of_focus_tab(&self) -> Option<i64> {
        self.context.id_of_focus_tab()
    }

    fn id_path_of_focus_tab(&self) -> Option<String> {
        match self.context.id_path_of_focus_tab() {
            Some(expr) => Some(format!("{}", expr)),
            None => None,
        }
    }

    fn focus_tab_name(&self, name: &str) -> bool {
        if let Some(idwin) = self.context.id_window_with_tab_title(&name) {
            self.context.focus_window_id(idwin);
            return true;
        }
        return false;
    }

    fn create_new_tab(&mut self, name: &str) {
        self.context.launch_shell_in_new_tab_name(&name);
    }

    fn change_tab_title(&self, name: &str) {
        self.context.set_tab_title(&name);
    }

    fn change_tab_color(&self, color: kitty::Tabcolor) {
        self.context.set_tab_color(color);
    }
}
