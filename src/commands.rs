use crate::application::{Application, HISTORY_FILE};
use crate::openai;

use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Select, MultiSelect};
use std::fs::remove_file;
use std::process;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub enum CommandError {
    CommandNotFound,
    InvalidModel,
}

pub trait Command {
    fn handle_command(&self, registry: &CommandRegistry, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError>;
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

    pub fn get_available_commands(&self) -> Vec<&'static str> {
        let mut v = Vec::<&'static str>::new();
        for key in self.commands.keys().into_iter() {
            v.push(key)
        }
        v
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

        self.register_command("delete", CommandDelete);
        self.register_command("del", CommandDelete);

        self.register_command("help", CommandHelp);

        self.register_command("set_model", CommandSetModel);
        self.register_command("setmodel", CommandSetModel);
        self.register_command("model", CommandSetModel);
    }

    pub fn execute_command(&self, name: &str, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        match self.commands.get(&name) {
            Some(x) => {
                x.handle_command(self, args, app)
            },
            None => Err(CommandError::CommandNotFound),
        }
    }
}

struct CommandExit;
impl Command for CommandExit {
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        process::exit(0);
    }
}

struct CommandClear;
impl Command for CommandClear {
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        println!("\x1B[2J\x1B[1;1H");
        Ok(())
    }
}

struct CommandCopy;
impl Command for CommandCopy {
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
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
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
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
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        if let Err(e) = remove_file(HISTORY_FILE) {
            eprintln!("Failed to clear history: {}", e);
        } else {
            println!("History cleared.");
        }
        Ok(())
    }
}

struct CommandDelete;
impl Command for CommandDelete {
    fn handle_command(&self, _registry: &CommandRegistry, _args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let app = app.borrow_mut();
        let shared_context = &app.context;
        let messages = app.tokio_rt.block_on(async {
            let locked = shared_context.lock().await;
            locked.clone()
        });

        let mut messages_choice = Vec::<String>::new();
        for msg in messages {
            let msg = format!("{}: {}", msg.role, msg.content);
            messages_choice.push(msg);
        }

        let mut selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select messages to delete")
            .items(&messages_choice)
            .interact()
            .unwrap();
        selections.sort_by(|a, b| b.cmp(a));

        app.tokio_rt.block_on(async {
            let mut locked = shared_context.lock().await;
            for i in selections {
                locked.remove(i);
            }
            locked.clone()
        });

        Ok(())
    }
}

struct CommandHelp;
impl Command for CommandHelp {
    fn handle_command(&self, registry: &CommandRegistry, _args: Vec<&str>, _app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        println!("Available commands:");
        for name in registry.get_available_commands() {
            println!("- {}", name);
        }
        Ok(())
    }
}

struct CommandSetModel;
impl Command for CommandSetModel {
    fn handle_command(&self, _registry: &CommandRegistry, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();

        let model_idx;
        if args.len() != 0 {
            match openai::AVAILABLE_MODELS.iter().position(|&r| r == args[0])  {
                Some(x) => {
                    model_idx = x
                },
                None => {
                    return Err(CommandError::InvalidModel);
                }
            };
        } else {
            let initial = openai::AVAILABLE_MODELS.iter().position(|&r| r == app.model).unwrap();
            model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Select a model to use. You are using {}.", app.model))
                .items(&openai::AVAILABLE_MODELS)
                .default(initial)
                .interact()
                .unwrap();
        }

        app.model = openai::AVAILABLE_MODELS[model_idx];
        println!("Model changed to {}!", app.model);
        Ok(())
    }
}

