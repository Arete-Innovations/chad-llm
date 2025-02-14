use crate::application::{Application, HISTORY_FILE};
use crate::openai;

use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Select, MultiSelect, Completion, Editor};
use fuzzy_matcher::clangd::fuzzy_match;

use std::fs::remove_file;
use std::process;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

impl Completion for CommandRegistry {
    fn get(&self, input: &str) -> Option<String> {
        let inp = input.to_string();
        let inp = inp.strip_prefix("/")?;
        let mut cmds: Vec<(&str, i64)> = self
            .get_available_commands()
            .into_iter()
            .map(|cmd| (cmd, fuzzy_match(&cmd, &inp)))
            .filter(|(_, score)| score.is_some())
            .map(|(cmd, score)| (cmd, score.unwrap()))
            .collect();
        cmds.sort_by(|(_, a), (_, b)| a.cmp(b));
        if cmds.is_empty() {
            None
        } else {
            Some(format!("/{}", cmds[0].0.to_string()))
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    CommandNotFound,
    InvalidModel,
    UpdateFailed,
    InvalidSystemPrompt,
    Aborted,
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
        self.register_command("clear_history", CommandClearHistory);
        self.register_command("delete", CommandDelete);
        self.register_command("help", CommandHelp);
        self.register_command("set_model", CommandSetModel);
        self.register_command("system_edit", CommandSystemEdit);
        self.register_command("system_remove", CommandSystemRemove);
        self.register_command("system_use", CommandSystemUse);
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

        let mut available_models: Vec<String> = vec![];

        app.tokio_rt.block_on(async {
            available_models = match openai::get_models().await {
                Some(x) => x,
                None => {
                    println!("Failed to fetch available models from OpenAI.");
                    openai::AVAILABLE_MODELS.iter().map(|m| m.to_string()).collect()
                },
            }
        });

        let model_idx;
        if args.len() != 0 {
            match available_models.iter().position(|r| r == args[0])  {
                Some(x) => {
                    model_idx = x
                },
                None => {
                    return Err(CommandError::InvalidModel);
                }
            };
        } else {
            let initial = available_models.iter().position(|r| *r == app.model).unwrap();
            model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Select a model to use. You are using {}.", app.model))
                .items(&available_models)
                .default(initial)
                .interact()
                .unwrap();
        }

        app.model = available_models[model_idx].clone();
        println!("Model changed to {}!", app.model);
        Ok(())
    }
}

struct CommandSystemEdit;
impl Command for CommandSystemEdit {
    fn handle_command(&self, _registry: &CommandRegistry, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();
        let available = app.system_prompts.get_available();

        let name: String;
        if args.len() == 0 {
            let active = app.active_system_prompt.clone();
            let initial = available.iter().position(|r| *r == *active).unwrap_or(0);
            let model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Select a system prompt to edit. You are using {:?}.", app.active_system_prompt))
                .items(&available)
                .default(initial)
                .interact()
                .unwrap();
            name = available.get(model_idx).unwrap().clone();
        } else {
            name = args.get(0).unwrap().to_string();
        }

        let existing_data = match app.system_prompts.get(&name) {
            Some(x) => x.clone(),
            _ => "You are a helpful virtual assistant.".to_string(),
        };

        if let Some(inp) = Editor::new().edit(&existing_data).unwrap() {
            match app.system_prompts.update_or_create(&name, &inp) {
                Ok(_) => {println!("Prompt updated."); Ok(())}
                Err(e) => {
                    println!("Failed to update. Reason: {}", e);
                    Err(CommandError::UpdateFailed)
                }
            }
        } else {
            Err(CommandError::Aborted)
        }
    }
}

struct CommandSystemRemove;
impl Command for CommandSystemRemove {
    fn handle_command(&self, _registry: &CommandRegistry, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();
        let available = app.system_prompts.get_available();

        let name;
        if args.len() == 0 {
            let active = app.active_system_prompt.clone();
            let initial = available.iter().position(|r| *r == *active).unwrap_or(0);
            let model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Select a system prompt to remove. You are using {:?}.", app.active_system_prompt))
                .items(&available)
                .default(initial)
                .interact()
                .unwrap();
            name = (*available.get(model_idx).unwrap()).clone();
        } else {
            name = args.get(0).unwrap().to_string();
        }

        app.system_prompts.remove(&name);

        Ok(())
    }
}

struct CommandSystemUse;
impl Command for CommandSystemUse {
    fn handle_command(&self, _registry: &CommandRegistry, args: Vec<&str>, app: Rc<RefCell<Application>>) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();
        let available = app.system_prompts.get_available();

        let name;
        if args.len() == 0 {
            let active = app.active_system_prompt.clone();
            let initial = available.iter().position(|r| *r == *active).unwrap_or(0);
            let model_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Select a system prompt to remove. You are using {:?}.", app.active_system_prompt))
                .items(&available)
                .default(initial)
                .interact()
                .unwrap();
            name = (*available.get(model_idx).unwrap()).clone();
        } else {
            name = args.get(0).unwrap().to_string();
        }

        let contents = match app.system_prompts.get(&name) {
            Some(x) => Some(x.clone()),
            None => None,
        };
        let contents = match contents {
            Some(x) => {
                app.active_system_prompt = name;
                x
            },
            None => {
                return Err(CommandError::InvalidSystemPrompt)
            }
        };

        let shared_context = &app.context;
        let _ = app.tokio_rt.block_on(async {
            let mut locked = shared_context.lock().await;
            openai::set_system_prompt(&mut locked, &contents);
            locked.clone()
        });

        Ok(())
    }
}

