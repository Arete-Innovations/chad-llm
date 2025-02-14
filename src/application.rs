use crate::openai;
use crate::history;
use crate::openai::AVAILABLE_MODELS;
use crate::system_prompt::SystemPrompts;

use tokio::runtime::Runtime;
use history::History;
use dialoguer::BasicHistory;
use dirs::data_dir;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Application {
    pub tokio_rt: Runtime,
    pub context: openai::SharedContext,
    pub cli_history: BasicHistory,
    pub session_history: History, // FIXME: Remove, we have SharedContext.
    pub code_blocks: Vec<String>,
    pub model: String,
    pub system_prompts: SystemPrompts,
    pub active_system_prompt: String,
}

pub const HISTORY_FILE: &str = "session_history.txt";

impl Application {
    pub fn new() -> Self {
        let mut app = Application {
            tokio_rt: Runtime::new().unwrap(),
            context: Arc::new(Mutex::new(Vec::new())),
            cli_history: BasicHistory::new().max_entries(99).no_duplicates(false),
            session_history: History::new(HISTORY_FILE),
            code_blocks: Vec::new(),
            model: AVAILABLE_MODELS[0].to_owned(),
            system_prompts: SystemPrompts::new(),
            active_system_prompt: "".to_owned(),
        };
        app.active_system_prompt = match app.system_prompts.get_available().iter().position(|r| r == "default") {
            None => app.system_prompts.get_available().get(0).unwrap().to_owned(),
            Some(_) => "default".to_owned(),
        };
        let mut path = data_dir().unwrap();
        path.push("chad-llm/");
        std::fs::create_dir(path.as_path()).unwrap();
        app
    }


}

