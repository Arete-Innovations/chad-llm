use crate::openai;
use crate::history;
use crate::MyCompletion;
use crate::openai::AVAILABLE_MODELS;

use tokio::runtime::Runtime;
use history::History;
use dialoguer::BasicHistory;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Application {
    pub tokio_rt: Runtime,
    pub context: openai::SharedContext,
    pub cli_history: BasicHistory,
    pub cli_completion: MyCompletion,
    pub session_history: History, // FIXME: Remove, we have SharedContext.
    pub code_blocks: Vec<String>,
    pub model: &'static str,
}

pub const HISTORY_FILE: &str = "session_history.txt";

impl Application {
    pub fn new() -> Self {
        Application {
            tokio_rt: Runtime::new().unwrap(),
            context: Arc::new(Mutex::new(Vec::new())),
            cli_history: BasicHistory::new().max_entries(99).no_duplicates(false),
            cli_completion: MyCompletion::default(),
            session_history: History::new(HISTORY_FILE),
            code_blocks: Vec::new(),
            model: AVAILABLE_MODELS[0]
        }
    }
}

