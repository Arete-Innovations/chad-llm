mod commands;
mod data;
mod history;
mod openai;
mod response;
mod models;
mod application;

use clipboard::{ClipboardContext, ClipboardProvider};
use data::MyCompletion;
use dialoguer::{theme::ColorfulTheme, Input};
use openai::send_request;
use std::io::Write;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;

fn main() -> ! {
    let gapp = Rc::new(RefCell::new(application::Application::new()));
    let mut command_registry = commands::CommandRegistry::new();
    command_registry.register_default_commands();

    // Load previous history entries
    match gapp.borrow_mut().session_history.load_history() {
        Ok(entries) => {
            for entry in entries {
                println!(" {}", entry);
            }
        }
        Err(e) => eprintln!("Failed to load history: {}", e),
    }

    loop {
        let mut input;
        {
            let mut app = gapp.borrow_mut();
            input = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt(whoami::realname()) // Add newline before prompt
                .completion_with(&mut app.cli_completion.clone())
                .history_with(&mut app.cli_history)
                .interact_text()
                .unwrap()
                .trim()
                .to_owned();
        }

        // Save the input to history
        {
            let app = gapp.borrow_mut();
            if let Err(e) = app.session_history.save_entry(&input) {
                eprintln!("Failed to save entry: {}", e);
            }
        }

        // Check if a command, and if so, then parse it.
        if input.starts_with('/') && input.len() > 1 {
            let mut args = Vec::<&str>::new();
            let mut name: &str = "<unknown command>";
            let mut first = true;

            input = input.strip_prefix('/').unwrap().to_owned();
            let input_cmd = input.clone();
            for arg in input_cmd.split(' ') {
                if arg == "" {
                    continue
                }
                if first {
                    name = arg
                } else {
                    args.push(arg)
                }
                first = false;
            }

            if name == "paste" {
                // FIXME: Register this as a command.
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
                let res = command_registry.execute_command(name, args, gapp.clone());
                match res {
                    Ok(()) => println!("Command executed successfuly!"),
                    Err(e) => println!("Failed to execute command. Reason: {:?}", e),
                }

                continue;
            }
        }

        let mut app = gapp.borrow_mut();
        // Now input contains the aggregated content
        let response_stream = app.tokio_rt.block_on(send_request(&input, Arc::clone(&app.context)));
        match response_stream {
            Ok(stream) => {
                let mut code_blocks = std::mem::take(&mut app.code_blocks);

                let response = app.tokio_rt.block_on(response::process_response(
                    Box::pin(stream),
                    &mut code_blocks,
                ));

                app.code_blocks = code_blocks;

                match response {
                    Ok(resp) => {
                        // Save the GPT response to history
                        if let Err(e) = app.session_history.save_response(&resp) {
                            eprintln!("Failed to save response: {}", e);
                        }
                    }
                    Err(err) => eprintln!("Failed to process response: {}", err),
                }
            }
            Err(err) => eprintln!("Request failed: {}", err),
        }

        println!();
        std::io::stdout().flush().unwrap();
    }
}
