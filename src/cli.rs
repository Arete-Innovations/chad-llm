use std::time::{Duration, Instant};
use std::{
    env::{self, VarError},
    io::{self, IsTerminal, Write},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use rand::{self, Rng};

pub struct CLI;

impl CLI {
    pub fn new() -> Self {
        if io::stdin().is_terminal() {
            terminal::enable_raw_mode().expect("Failed to set terminal to raw mode.");
        }
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

        let _ = terminal::disable_raw_mode();
        let status = std::process::Command::new(editor)
            .args([fp.to_str()?])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .unwrap();

        if !status.success() {
            return None;
        }
        let _ = terminal::enable_raw_mode();

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
        let mut selected_indices: Vec<usize> = selected.to_vec();
        let mut current_index = selected.first().copied().unwrap_or(0);
        let visible_count = 5.min(options.len());
        for _ in 0..visible_count {
            print!("\r\n");
        }

        let mut offset = if current_index >= visible_count {
            current_index + 1 - visible_count
        } else {
            0
        };

        let mut stdout = io::stdout();

        write!(std::io::stdout(), "{}", prompt).unwrap();

        fn clear(stdout: &mut io::Stdout, visible_count: usize) {
            execute!(stdout, terminal::Clear(ClearType::CurrentLine)).unwrap();
            for _ in 0..visible_count {
                execute!(
                    stdout,
                    terminal::Clear(ClearType::CurrentLine),
                    cursor::MoveDown(1)
                )
                .unwrap();
            }
            for _ in 0..visible_count {
                execute!(stdout, cursor::MoveUp(1)).unwrap();
            }
        }

        fn draw<T: ToString>(
            stdout: &mut io::Stdout,
            options: &[T],
            current_index: usize,
            selected_indices: &[usize],
            offset: usize,
            visible_count: usize,
        ) {
            clear(stdout, visible_count);

            for i in offset..(offset + visible_count).min(options.len()) {
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
                print!("{}\r\n", options[i].to_string());
            }
            stdout.flush().unwrap();
        }

        draw(
            &mut stdout,
            options,
            current_index,
            &selected_indices,
            offset,
            visible_count,
        );

        loop {
            if event::poll(Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Up => {
                            if current_index > 0 {
                                current_index -= 1;
                                if current_index < offset {
                                    offset -= 1;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if current_index < options.len() - 1 {
                                current_index += 1;
                                if current_index >= offset + visible_count {
                                    offset += 1;
                                }
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

                    draw(
                        &mut stdout,
                        options,
                        current_index,
                        &selected_indices,
                        offset,
                        visible_count,
                    );
                }
            }
        }

        for _ in 0..visible_count {
            execute!(std::io::stdout(), cursor::MoveUp(1)).unwrap();
        }

        clear(&mut std::io::stdout(), visible_count);

        stdout.flush().unwrap();

        selected_indices
    }

    pub fn read_line(prompt: &str) -> Option<String> {
        let mut last_time = Instant::now();
        let mut typed_chars = 0;
        let mut read_so_far = String::new();
        let mut in_paste = false;
        let mut cur_pos: usize = 0;

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

                            read_so_far.insert(cur_pos, c);
                            cur_pos += 1;

                            print!("\r{}{}", prompt, read_so_far);
                            execute!(
                                io::stdout(),
                                cursor::MoveToColumn((prompt.len() + cur_pos) as u16)
                            )
                            .unwrap();
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Left => {
                            if cur_pos > 0 {
                                cur_pos -= 1;
                                execute!(io::stdout(), cursor::MoveLeft(1)).unwrap();
                            }
                        }
                        KeyCode::Right => {
                            if cur_pos < read_so_far.len() {
                                cur_pos += 1;
                                execute!(io::stdout(), cursor::MoveRight(1)).unwrap();
                            }
                        }
                        KeyCode::Backspace => {
                            if cur_pos > 0 {
                                read_so_far.remove(cur_pos - 1);
                                cur_pos -= 1;

                                print!("\r{}{}", prompt, read_so_far);
                                print!(" ");
                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn((prompt.len() + cur_pos) as u16)
                                )
                                .unwrap();
                                io::stdout().flush().unwrap();
                            }
                        }
                        KeyCode::Delete => {
                            if cur_pos < read_so_far.len() {
                                read_so_far.remove(cur_pos);

                                print!("\r{}{}", prompt, read_so_far);
                                print!(" ");
                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn((prompt.len() + cur_pos) as u16)
                                )
                                .unwrap();
                                io::stdout().flush().unwrap();
                            }
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
        if io::stdin().is_terminal() {
            let _ = terminal::disable_raw_mode();
        }
    }
}
