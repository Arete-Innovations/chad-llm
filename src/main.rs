#![allow(dead_code)]

mod application;
mod cli;
mod commands;
mod history;
mod models;
mod openai;
mod response;
mod system_prompt;

use cli::{CLI, ReadLine};
use clipboard::{ClipboardContext, ClipboardProvider};
use openai::send_request;
use std::cell::RefCell;
use std::io::{self, BufRead, IsTerminal, Write};
use std::rc::Rc;
use std::sync::Arc;

fn main() {
    let gapp = Rc::new(RefCell::new(application::Application::new()));
    let mut command_registry = commands::CommandRegistry::new();
    command_registry.register_default_commands();

    if io::stdin().is_terminal() {
        // Load previous history entries
        match gapp.borrow_mut().session_history.load_history() {
            Ok(entries) => {
                for entry in entries {
                    print!(" {}\r\n", entry);
                }
            }
            Err(e) => eprint!("Failed to load history: {}\r\n", e),
        }
    }

    loop {
        let mut input = String::new();
        if !io::stdin().is_terminal() {
            for line in io::stdin().lock().lines() {
                input.push_str(&line.unwrap());
            }
        } else {
            {
                input = match ReadLine::new()
                    .prompt(&format!("[$green]{} [$/]> ", whoami::realname()))
                    .completion(&command_registry)
                    .run()
                    {
                        Some(x) => x,
                        None => continue,
                    };
                //    .history_with(&mut app.cli_history)
            }

            // Save the input to history
            {
                let app = gapp.borrow_mut();
                if let Err(e) = app.session_history.save_entry(&input) {
                    eprint!("Failed to save entry: {}\r\n", e);
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
                        continue;
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
                                ReadLine::new().prompt("Add additional details").run().unwrap();

                            // Aggregate the clipboard content and additional input
                            input.push_str(&paste_content);
                            input.push_str(&additional_input);
                        }
                        Err(err) => eprint!("Failed to read clipboard: {}\r\n", err),
                    }
                } else if name == "editor" {
                    if let Some(inp) = CLI::editor("") {
                        input = inp
                    } else {
                        print!("Aborted!\r\n");
                        continue;
                    }
                } else if name == "quit" || name == "exit" {
                    break;
                } else {
                    let res = command_registry.execute_command(name, args, gapp.clone());
                    match res {
                        Ok(()) => print!("Command executed successfuly!\r\n"),
                        Err(e) => print!("Failed to execute command. Reason: {:?}\r\n", e),
                    }

                    continue;
                }
            }
        }

        let mut app = gapp.borrow_mut();
        let response_stream =
            app.tokio_rt
                .block_on(send_request(&input, Arc::clone(&app.context), &app.model));
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
                            eprint!("Failed to save response: {}\r\n", e);
                        }
                    }
                    Err(err) => eprint!("Failed to process response: {}\r\n", err),
                }
            }
            Err(err) => eprint!("Request failed: {}\r\n", err),
        }

        print!("\r\n");
        std::io::stdout().flush().unwrap();

        if !io::stdin().is_terminal() {
            break;
        }
    }
}
