mod commands;
mod data;
mod history;
mod openai;
mod response;
mod models;


use clipboard::{ClipboardContext, ClipboardProvider};
use commands::{handle_command, is_command};
use data::MyCompletion;
use dialoguer::{theme::ColorfulTheme, BasicHistory, Input};
use history::History; // Import the History struct
use openai::send_request;
use std::io::Write;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

fn main() {
    let context = Arc::new(Mutex::new(Vec::new()));
    let mut history = BasicHistory::new().max_entries(99).no_duplicates(false);
    let rt = Runtime::new().unwrap();
    let completion = MyCompletion::default();
    let session_history = History::new("session_history.txt"); // Initialize history

    // Load previous history entries
    match session_history.load_history() {
        Ok(entries) => {
            for entry in entries {
                println!(" {}", entry);
            }
        }
        Err(e) => eprintln!("Failed to load history: {}", e),
    }

    loop {
        let mut code_blocks: Vec<String> = Vec::new(); // Reset code_blocks for each interaction

        let mut input = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt(whoami::realname()) // Add newline before prompt
            .completion_with(&completion)
            .history_with(&mut history)
            .interact_text()
            .unwrap();

        // Save the input to history
        if let Err(e) = session_history.save_entry(&input) {
            eprintln!("Failed to save entry: {}", e);
        }

        // Check if the input is a command
        if is_command(&input) {
            if input.trim() == "/paste" {
                // If the command was /paste, append the clipboard content to the input
                let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
                match clipboard.get_contents() {
                    Ok(paste_content) => {
                        print!("\n{}", paste_content); // Print the clipboard content
                        std::io::stdout().flush().unwrap();

                        let additional_input =
                            Input::<String>::with_theme(&ColorfulTheme::default())
                                .with_prompt("Add additional details")
                                .interact_text()
                                .unwrap();

                        // Aggregate the clipboard content and additional input
                        input.push_str(&paste_content);
                        input.push_str(&additional_input);
                    }
                    Err(err) => eprintln!("Failed to read clipboard: {}", err),
                }
            } else {
                handle_command(&input, &code_blocks, "session_history.txt"); // Pass the history file path
                continue; // Skip to the next loop iteration
            }
        }

        // Now input contains the aggregated content
        let response_stream = rt.block_on(send_request(&input, Arc::clone(&context)));
        match response_stream {
            Ok(stream) => {
                let response = rt.block_on(response::process_response(
                    Box::pin(stream),
                    &mut code_blocks,
                ));

                match response {
                    Ok(resp) => {
                        // Save the GPT response to history
                        if let Err(e) = session_history.save_response(&resp) {
                            eprintln!("Failed to save response: {}", e);
                        }
                    }
                    Err(err) => eprintln!("Failed to process response: {}", err),
                }
            }
            Err(err) => eprintln!("Request failed: {}", err),
        }

        if !code_blocks.is_empty() {
            if let Ok(command_input) = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter command")
                .interact_text()
            {
                handle_command(&command_input, &code_blocks, "session_history.txt");
            }
        }
        println!(); // Ensure new line after each interaction
        std::io::stdout().flush().unwrap(); // Ensure stdout is flushed after each interaction
    }
}
