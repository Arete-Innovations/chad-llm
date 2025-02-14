use std::time::{Duration, Instant};
use std::{
    env::{self, VarError},
    io::{self, Write},
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
};
use rand::{self, Rng};

pub struct CLI;

impl CLI {
    pub fn new() -> Self {
        terminal::enable_raw_mode().expect("Failed to set terminal to raw mode.");
        Self {}
    }

    pub fn clear() {
        print!("\x1B[2J\x1B[H");
    }

    fn get_editor() -> Result<String, VarError> {
        match env::var("VISUAL") {
            Ok(result) => return Ok(result),
            Err(VarError::NotPresent) => {}
            Err(error) => return Err(error),
        }

        match env::var("EDITOR") {
            Ok(result) => return Ok(result),
            Err(VarError::NotPresent) => {}
            Err(error) => return Err(error),
        }

        Ok("vi".to_string())
    }

    pub fn editor(original: &str) -> Option<String> {
        let mut fp = tempfile::env::temp_dir();
        let s: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        fp.push(format!(".llm_tmp_{}", s));
        match std::fs::write(&fp, original) {
            Ok(()) => {}
            Err(_) => return None,
        }

        let editor = match Self::get_editor() {
            Ok(e) => e,
            Err(_) => return None,
        };

        let out = std::process::Command::new(editor)
            .args([fp.to_str()?])
            .output()
            .unwrap();
        if !out.status.success() {
            return None;
        }

        let new = match std::fs::read(fp) {
            Ok(s) => s,
            Err(_) => return None,
        };
        let new = String::from_utf8(new).unwrap();

        if new == original {
            None
        } else {
            Some(new)
        }
    }

    pub fn select<T: ToString>(
        prompt: &str,
        options: &[T],
        single: bool,
        selected: &[usize],
    ) -> Vec<usize> {
        let mut selected_indices = Vec::new();
        let mut current_index = 0;

        print!("{}\n", prompt);
        for (i, option) in options.iter().enumerate() {
            if i == current_index {
                print!("> ");
            } else {
                print!("  ");
            }
            print!("{}\r\n", option.to_string());
        }
        io::stdout().flush().unwrap();

        loop {
            if event::poll(Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Up => {
                            if current_index > 0 {
                                current_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if current_index < options.len() - 1 {
                                current_index += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if single {
                                selected_indices.clear();
                                selected_indices.push(current_index);
                            } else if selected_indices.contains(&current_index) {
                                selected_indices.retain(|&x| x != current_index);
                            } else {
                                selected_indices.push(current_index);
                            }
                        }
                        KeyCode::Enter => {
                            if selected_indices.is_empty() && single {
                                selected_indices.push(current_index);
                            }
                            break;
                        }
                        KeyCode::Esc => {
                            selected_indices.clear();
                            break;
                        }
                        _ => {}
                    }

                    CLI::clear();

                    print!("{}\n", prompt);
                    for (i, option) in options.iter().enumerate() {
                        if i == current_index {
                            print!("> ");
                        } else {
                            print!("  ");
                        }
                        if selected_indices.contains(&i) {
                            print!("[x] ");
                        } else {
                            print!("[ ] ");
                        }
                        print!("{}\r\n", option.to_string());
                    }
                    io::stdout().flush().unwrap();
                }
            }
        }

        selected_indices
    }

    pub fn read_line(prompt: &str) -> Option<String> {
        let mut last_time = Instant::now();
        let mut typed_chars = 0;
        let mut read_so_far = String::new();
        let mut in_paste = false;
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        loop {
            if event::poll(Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    let now = Instant::now();
                    let elapsed = now.duration_since(last_time).as_millis();

                    match key_event.code {
                        KeyCode::Char(c) => {
                            if typed_chars > 5 && elapsed < 10 {
                                in_paste = true;
                            }

                            last_time = now;
                            typed_chars += 1;

                            if c == '\n' {
                                print!("\r\n");
                            } else {
                                print!("{}", c);
                            }
                            read_so_far.push(c);
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Enter => {
                            print!("\r\n");
                            io::stdout().flush().unwrap();

                            if !in_paste && elapsed > 20 {
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            read_so_far.clear();
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Some(read_so_far)
    }
}

impl Drop for CLI {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}
