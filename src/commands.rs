use crate::application::{Application, HISTORY_FILE};
use crate::cli::CLI;
use crate::openai;

use clipboard::{ClipboardContext, ClipboardProvider};
//use fuzzy_matcher::clangd::fuzzy_match;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::remove_file;
use std::rc::Rc;

fn get_input_or_select<'a>(
    args: &[&str],
    available: &'a [&'a str],
    prompt: &str,
    default: Option<&str>,
) -> Option<String> {
    if let Some(&arg) = args.get(0) {
        return Some(arg.to_string());
    }

    let initial = default
        .and_then(|d| available.iter().position(|&r| r == d))
        .unwrap_or(0);

    let v = CLI::select(prompt, available, true, &[initial]);
    if v.is_empty() {
        return None;
    }
    Some(available[v[0]].to_string())
}

//impl Completion for CommandRegistry {
//    fn get(&self, input: &str) -> Option<String> {
//        let inp = input.to_string();
//        let inp = inp.strip_prefix("/")?;
//        let mut cmds: Vec<(&str, i64)> = self
//            .get_available_commands()
//            .into_iter()
//            .map(|cmd| (cmd, fuzzy_match(&cmd, &inp)))
//            .filter(|(_, score)| score.is_some())
//            .map(|(cmd, score)| (cmd, score.unwrap()))
//            .collect();
//        cmds.sort_by(|(_, a), (_, b)| a.cmp(b));
//        if cmds.is_empty() {
//            None
//        } else {
//            Some(format!("/{}", cmds[0].0.to_string()))
//        }
//    }
//}

#[derive(Debug)]
pub enum CommandError {
    CommandNotFound,
    InvalidModel,
    UpdateFailed,
    InvalidSystemPrompt,
    Aborted,
}

pub trait Command {
    fn handle_command(
        &self,
        registry: &CommandRegistry,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError>;
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

    pub fn execute_command(
        &self,
        name: &str,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        match self.commands.get(&name) {
            Some(x) => x.handle_command(self, args, app),
            None => Err(CommandError::CommandNotFound),
        }
    }
}

struct CommandExit;
impl Command for CommandExit {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        _app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        Ok(())
    }
}

struct CommandClear;
impl Command for CommandClear {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        _app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        print!("\x1B[2J\x1B[1;1H\r\n");
        Ok(())
    }
}

struct CommandCopy;
impl Command for CommandCopy {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let app = app.borrow_mut();
        if app.code_blocks.is_empty() {
            print!("No code blocks to copy.\r\n");
            return Ok(());
        }

        let selections: Vec<&str> = app.code_blocks.iter().map(|s| s.as_str()).collect();
        let selection = *CLI::select("Select code block to copy", &selections, true, &[0])
            .get(0)
            .unwrap_or(&0);

        let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
        clipboard
            .set_contents(app.code_blocks[selection].clone())
            .unwrap();
        print!("Code block copied to clipboard\r\n");
        Ok(())
    }
}

struct CommandCopyAll;
impl Command for CommandCopyAll {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let app = app.borrow_mut();
        if app.code_blocks.is_empty() {
            print!("No code blocks to copy.\r\n");
            return Ok(());
        }

        let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
        let all_code = app.code_blocks.join("\n\n");
        clipboard.set_contents(all_code.clone()).unwrap();
        print!("All code blocks copied to clipboard\r\n");
        Ok(())
    }
}

struct CommandClearHistory;
impl Command for CommandClearHistory {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        _app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        if let Err(e) = remove_file(HISTORY_FILE) {
            eprint!("Failed to clear history: {}\r\n", e);
        } else {
            print!("History cleared.\r\n");
        }
        Ok(())
    }
}

struct CommandDelete;
impl Command for CommandDelete {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        _args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
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

        let mut selections = CLI::select("Select messages to delete", &messages_choice, false, &[]);
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
    fn handle_command(
        &self,
        registry: &CommandRegistry,
        _args: Vec<&str>,
        _app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        print!("Available commands:\r\n");
        for name in registry.get_available_commands() {
            print!("- {}\r\n", name);
        }
        Ok(())
    }
}

struct CommandSetModel;
impl Command for CommandSetModel {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();

        let mut available_models: Vec<String> = vec![];

        app.tokio_rt.block_on(async {
            available_models = match openai::get_models().await {
                Some(x) => x,
                None => {
                    print!("Failed to fetch available models from OpenAI.\r\n");
                    openai::AVAILABLE_MODELS
                        .iter()
                        .map(|m| m.to_string())
                        .collect()
                }
            }
        });

        let model_idx;
        if args.len() != 0 {
            match available_models.iter().position(|r| r == args[0]) {
                Some(x) => model_idx = x,
                None => {
                    return Err(CommandError::InvalidModel);
                }
            };
        } else {
            let initial = available_models
                .iter()
                .position(|r| *r == app.model)
                .unwrap();
            model_idx = *CLI::select(
                &format!("Select a model to use. You are using {}.", app.model),
                &available_models,
                true,
                &[initial],
            )
            .get(0)
            .unwrap_or(&0);
        }

        app.model = available_models[model_idx].clone();
        print!("Model changed to {}!\r\n", app.model);
        Ok(())
    }
}

struct CommandSystemEdit;
impl Command for CommandSystemEdit {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();

        let available_prompts = app.system_prompts.get_available();
        let name = match get_input_or_select(
            &args,
            &available_prompts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            "Select a system prompt:",
            Some(&app.active_system_prompt),
        ) {
            Some(name) => name,
            None => return Err(CommandError::Aborted),
        };

        let existing_data = match app.system_prompts.get(&name) {
            Some(x) => x.clone(),
            _ => "You are a helpful virtual assistant.".to_string(),
        };

        if let Some(inp) = CLI::editor(&existing_data) {
            match app.system_prompts.update_or_create(&name, &inp) {
                Ok(_) => {
                    print!("Prompt updated.\r\n");
                    Ok(())
                }
                Err(e) => {
                    print!("Failed to update. Reason: {}\r\n", e);
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
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();

        let available_prompts = app.system_prompts.get_available();
        let name = match get_input_or_select(
            &args,
            &available_prompts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            "Select a system prompt:",
            Some(&app.active_system_prompt),
        ) {
            Some(name) => name,
            None => return Err(CommandError::Aborted),
        };

        app.system_prompts.remove(&name);

        Ok(())
    }
}

struct CommandSystemUse;
impl Command for CommandSystemUse {
    fn handle_command(
        &self,
        _registry: &CommandRegistry,
        args: Vec<&str>,
        app: Rc<RefCell<Application>>,
    ) -> Result<(), CommandError> {
        let mut app = app.borrow_mut();

        let available_prompts = app.system_prompts.get_available();
        let name = match get_input_or_select(
            &args,
            &available_prompts
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            "Select a system prompt:",
            Some(&app.active_system_prompt),
        ) {
            Some(name) => name,
            None => return Err(CommandError::Aborted),
        };

        let contents = match app.system_prompts.get(&name) {
            Some(x) => Some(x.clone()),
            None => None,
        };
        let contents = match contents {
            Some(x) => {
                app.active_system_prompt = name;
                x
            }
            None => return Err(CommandError::InvalidSystemPrompt),
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
