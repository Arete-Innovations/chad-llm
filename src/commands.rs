use crate::application::{Application, HISTORY_FILE};

use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Select};
use std::fs::remove_file;
use std::process;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub enum CommandError {
    CommandNotFound,
}

pub trait Command {
    fn handle_command(&self, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError>;
}

pub struct CommandRegistry {
    commands: HashMap<&'static str, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register_command<C: Command + 'static>(&mut self, name: &'static str, command: C) {
        self.commands.insert(name, Box::new(command));
    }

    pub fn register_default_commands(&mut self) {
        self.register_command("exit", CommandExit);
        self.register_command("quit", CommandExit);

        self.register_command("clear", CommandClear);
        self.register_command("cls", CommandClear);

        self.register_command("copy", CommandCopy);

        self.register_command("copy_all", CommandCopyAll);
        self.register_command("copyall", CommandCopyAll);

        self.register_command("clear_history", CommandClearHistory);
        self.register_command("clearhistory", CommandClearHistory);
        self.register_command("clear_h", CommandClearHistory);
        self.register_command("clearh", CommandClearHistory);
    }

    pub fn execute_command(&self, name: &str, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        match self.commands.get(&name) {
            Some(x) => {
                x.handle_command(args, app)
            },
            None => Err(CommandError::CommandNotFound),
        }
    }
}

struct CommandExit;
impl Command for CommandExit {
    fn handle_command(&self, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        process::exit(0);
    }
}

struct CommandClear;
impl Command for CommandClear {
    fn handle_command(&self, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        println!("\x1B[2J\x1B[1;1H");
        Ok(())
    }
}

struct CommandCopy;
impl Command for CommandCopy {
    fn handle_command(&self, _args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let app = app.borrow_mut();
        if app.code_blocks.is_empty() {
            println!("No code blocks to copy.");
            return Ok(())
        }

        let selections: Vec<&str> = app.code_blocks.iter().map(|s| s.as_str()).collect();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select code block to copy")
            .items(&selections)
            .default(0)
            .interact()
            .unwrap();

        let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
        clipboard
            .set_contents(app.code_blocks[selection].clone())
            .unwrap();
        println!("Code block copied to clipboard");
        Ok(())
    }
}

struct CommandCopyAll;
impl Command for CommandCopyAll {
    fn handle_command(&self, _args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let app = app.borrow_mut();
        if app.code_blocks.is_empty() {
            println!("No code blocks to copy.");
            return Ok(())
        }

        let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
        let all_code = app.code_blocks.join("\n\n");
        clipboard.set_contents(all_code.clone()).unwrap();
        println!("All code blocks copied to clipboard");
        Ok(())
    }
}

struct CommandClearHistory;
impl Command for CommandClearHistory {
    fn handle_command(&self, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        if let Err(e) = remove_file(HISTORY_FILE) {
            eprintln!("Failed to clear history: {}", e);
        } else {
            println!("History cleared.");
        }
        Ok(())
    }
}

