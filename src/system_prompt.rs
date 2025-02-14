use dirs::data_dir;
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::error::Error;

const FILE_NAME: &'static str = "chad-llm/system_prompts.json";

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

impl Error for SystemPromptsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
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
        if this.prompts.is_empty() {
            this.update_or_create("default", "You are a helpful assistant.").unwrap();
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

    pub fn update(&mut self, name: &str, contents: &str) -> Result<(), Box<dyn Error>>  {
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

    pub fn update_or_create(&mut self, name: &str, contents: &str) -> Result<(), Box<dyn Error>> {
        match self.update(name, contents) {
            Ok(()) => Ok(()),
            Err(_) => {
                self.prompts.insert(name.to_owned(), contents.to_owned());
                Ok(())
            }
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.prompts.remove(name);
    }

    fn get_file_path() -> std::path::PathBuf {
        let mut path = data_dir().unwrap();
        path.push("chad-llm/");
        path.push(FILE_NAME);
        path
    }

    fn import(&mut self) -> Result<(), Box<dyn Error>> {
        let path = Self::get_file_path();
        let path = path.as_path();
        let file_contents = std::fs::read_to_string(path)?;
        let read: Self = serde_json::from_str(&file_contents)?;

        self.prompts = read.prompts.clone();

        Ok(())
    }

    fn export(&self) -> Result<(), Box<dyn Error>> {
        let path = Self::get_file_path();
        let path = path.as_path();

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

