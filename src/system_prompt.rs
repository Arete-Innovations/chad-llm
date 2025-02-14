use dirs::data_dir;
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::path::Path;

const FILE_NAME: &'static str = "system_prompts.json";

#[derive(Serialize, Deserialize)]
pub struct SystemPrompts {
    prompts: HashMap<String, String>,
}

#[derive(Debug)]
enum SystemPromptsError {
    FailedToFindPrompt,
}

impl std::fmt::Display for SystemPromptsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SystemPromptsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl SystemPrompts {
    pub fn new() -> Self {
        let mut this = Self {
            prompts: HashMap::new(),
        };
        if let Err(err) = this.import() {
            println!("Failed to import system prompts. Reason: {}", err);
        }
        this
    }

    pub fn get_available(&self) -> Vec<String> {
        let mut vec = vec![];
        for (k, _) in self.prompts.iter() {
            vec.push(k.to_string());
        }
        vec
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        return self.prompts.get(name);
    }

    pub fn update(&mut self, name: &str, contents: &str) -> Result<(), Box<dyn std::error::Error>>  {
        match self.prompts.get_mut(name) {
            None => {
                return Err(Box::new(SystemPromptsError::FailedToFindPrompt))
            }
            Some(string) => {
                *string = contents.to_string();
                self.export()
            }
        }
    }

    fn import(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(FILE_NAME);
        let file_contents = std::fs::read_to_string(path)?;
        let read: Self = serde_json::from_str(&file_contents)?;

        self.prompts = read.prompts.clone();

        Ok(())
    }

    fn export(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(FILE_NAME);
        let j = serde_json::to_string(&self)?;
        std::fs::write(path, j)?;
        Ok(())
    }
}

impl Drop for SystemPrompts {
    fn drop(&mut self) {
        self.export().unwrap();
    }
}

