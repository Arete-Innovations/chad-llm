use fuzzy_matcher::clangd::fuzzy_match;
use std::ascii::AsciiExt;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::{
    env::{self, VarError},
    io::{self, IsTerminal, Write},
};

use crossterm::cursor::MoveUp;
use crossterm::{
    cursor,
    event::KeyModifiers,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use rand::{self, Rng};

pub trait History<T> {
    fn read(&self, pos: usize) -> Option<String>;
    fn write(&mut self, val: &T);
}

pub struct BasicHistory {
    deque: VecDeque<String>,
}

impl BasicHistory {
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }
}

impl<T: ToString> History<T> for BasicHistory {
    fn read(&self, pos: usize) -> Option<String> {
        self.deque.get(pos).cloned()
    }

    fn write(&mut self, val: &T) {
        let val = val.to_string();
        self.deque.push_front(val);
    }
}

pub struct ReadLine<'a, T> {
    prompt: String,
    history: Option<&'a mut dyn History<T>>,
    completion: Option<&'a dyn Completion>,
}

pub trait Completion {
    fn get(&self, input: &str) -> Option<String>;
}

impl<'a, T> ReadLine<'a, T>
where
    T: std::str::FromStr,
{
    pub fn new() -> Self {
        Self {
            prompt: String::new(),
            history: None,
            completion: None,
        }
    }

    pub fn prompt<A: ToString>(mut self, prompt: A) -> Self {
        self.prompt = vari::format(&prompt.to_string());
        self
    }

    pub fn history(mut self, history: &'a mut dyn History<T>) -> Self {
        self.history = Some(history);
        self
    }

    pub fn completion<C>(mut self, completion: &'a C) -> Self
    where
        C: Completion,
    {
        self.completion = Some(completion);
        self
    }

    pub fn run(&mut self) -> Option<T>
    where
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        terminal::enable_raw_mode().expect("Failed to set terminal to raw mode.");

        let mut last_time = Instant::now();
        let mut typed_chars = 0;
        let mut read_so_far = String::new();
        let mut in_paste = false;
        let mut cur_pos: usize = 0;
        let mut hist_pos: isize = -1;

        print!("{}", self.prompt);
        io::stdout().flush().unwrap();

        loop {
            if event::poll(Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    let now = Instant::now();
                    let elapsed = now.duration_since(last_time).as_millis();
                    if elapsed > 30 {
                        in_paste = false;
                    }

                    match key_event.code {
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            write!(std::io::stdout(), "^C\r\n").unwrap();
                            return None;
                        }
                        KeyCode::Char('w') | KeyCode::Backspace
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            if cur_pos > 0 {
                                let mut delete_start = cur_pos;
                                while delete_start > 0
                                    && read_so_far
                                        .chars()
                                        .nth(delete_start - 1)
                                        .map_or(false, |c| c.is_whitespace())
                                {
                                    delete_start -= 1;
                                }
                                while delete_start > 0
                                    && read_so_far
                                        .chars()
                                        .nth(delete_start - 1)
                                        .map_or(false, |c| !c.is_whitespace())
                                {
                                    delete_start -= 1;
                                }

                                read_so_far.replace_range(delete_start..cur_pos, "");
                                cur_pos = delete_start;

                                execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine))
                                    .unwrap();
                                write!(io::stdout(), "\r{}{}", self.prompt, read_so_far).unwrap();
                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn(
                                        (strip_ansi_escapes::strip(self.prompt.clone()).len()
                                            + cur_pos)
                                            as u16
                                    )
                                )
                                .unwrap();
                            }
                        }
                        KeyCode::Char('l')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            CLI::clear();
                            write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far).unwrap();
                        }
                        KeyCode::Char(c) => {
                            if typed_chars > 5 && elapsed < 10 {
                                in_paste = true;
                            }
                            last_time = now;
                            typed_chars += 1;

                            read_so_far.insert(cur_pos, c);
                            cur_pos += 1;

                            write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far).unwrap();

                            execute!(
                                io::stdout(),
                                cursor::MoveToColumn(
                                    (strip_ansi_escapes::strip(self.prompt.clone()).len() + cur_pos)
                                        as u16
                                )
                            )
                            .unwrap();
                        }
                        KeyCode::Tab => {
                            if let Some(completion) = self.completion {
                                let so_far: String = read_so_far.chars().take(cur_pos).collect();
                                let the_rest: String = read_so_far.chars().skip(cur_pos).collect();
                                if let Some(result) = completion.get(&so_far) {
                                    cur_pos = result.len();
                                    read_so_far = result + &the_rest;
                                    execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine))
                                        .unwrap();
                                    write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far)
                                        .unwrap();
                                }
                            }
                        }
                        KeyCode::Left if !key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            if cur_pos > 0 {
                                cur_pos -= 1;
                                execute!(io::stdout(), cursor::MoveLeft(1)).unwrap();
                            }
                        }
                        KeyCode::Right if !key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            if cur_pos < read_so_far.len() {
                                cur_pos += 1;
                                execute!(io::stdout(), cursor::MoveRight(1)).unwrap();
                            }
                        }
                        KeyCode::Left if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            if cur_pos > 0 {
                                while cur_pos > 0
                                    && read_so_far
                                        .chars()
                                        .nth(cur_pos - 1)
                                        .map_or(false, |c| c.is_whitespace())
                                {
                                    cur_pos -= 1;
                                }
                                while cur_pos > 0
                                    && read_so_far
                                        .chars()
                                        .nth(cur_pos - 1)
                                        .map_or(false, |c| !c.is_whitespace())
                                {
                                    cur_pos -= 1;
                                }

                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn(
                                        (strip_ansi_escapes::strip(self.prompt.clone()).len()
                                            + cur_pos)
                                            as u16
                                    )
                                )
                                .unwrap();
                            }
                        }
                        KeyCode::Right if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            if cur_pos < read_so_far.len() {
                                while cur_pos < read_so_far.len()
                                    && read_so_far
                                        .chars()
                                        .nth(cur_pos)
                                        .map_or(false, |c| !c.is_whitespace())
                                {
                                    cur_pos += 1;
                                }
                                while cur_pos < read_so_far.len()
                                    && read_so_far
                                        .chars()
                                        .nth(cur_pos)
                                        .map_or(false, |c| c.is_whitespace())
                                {
                                    cur_pos += 1;
                                }

                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn(
                                        (strip_ansi_escapes::strip(self.prompt.clone()).len()
                                            + cur_pos)
                                            as u16
                                    )
                                )
                                .unwrap();
                            }
                        }
                        KeyCode::Backspace => {
                            if cur_pos > 0 {
                                read_so_far.remove(cur_pos - 1);
                                cur_pos -= 1;

                                write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far)
                                    .unwrap();
                                print!(" ");
                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn(
                                        (strip_ansi_escapes::strip(self.prompt.clone()).len()
                                            + cur_pos)
                                            as u16
                                    )
                                )
                                .unwrap();
                                io::stdout().flush().unwrap();
                            }
                        }
                        KeyCode::Delete => {
                            if cur_pos < read_so_far.len() {
                                read_so_far.remove(cur_pos);

                                write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far)
                                    .unwrap();
                                print!(" ");
                                execute!(
                                    io::stdout(),
                                    cursor::MoveToColumn(
                                        (strip_ansi_escapes::strip(self.prompt.clone()).len()
                                            + cur_pos)
                                            as u16
                                    )
                                )
                                .unwrap();
                            }
                        }
                        KeyCode::Enter => {
                            print!("\r\n");
                            io::stdout().flush().unwrap();

                            if !in_paste {
                                break;
                            }
                        }
                        KeyCode::Up => {
                            if let Some(hist) = &self.history {
                                hist_pos += 1;
                                if let Some(value) = hist.read(hist_pos as usize) {
                                    cur_pos = value.len();
                                    read_so_far = value;
                                } else {
                                    hist_pos -= 1;
                                }
                                execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine))
                                    .unwrap();
                                write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far)
                                    .unwrap();
                            }
                        }
                        KeyCode::Down => {
                            if let Some(hist) = &self.history {
                                hist_pos -= 1;
                                if let Some(value) = hist.read(hist_pos as usize) {
                                    cur_pos = value.len();
                                    read_so_far = value;
                                } else {
                                    read_so_far = "".to_owned();
                                    cur_pos = 0;
                                    hist_pos = -1;
                                }
                                execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine))
                                    .unwrap();
                                write!(std::io::stdout(), "\r{}{}", self.prompt, read_so_far)
                                    .unwrap();
                            }
                        }
                        _ => {}
                    }
                    io::stdout().flush().unwrap();
                }
            }
        }
        io::stdout().flush().unwrap();

        terminal::disable_raw_mode().expect("Failed to remove terminal to raw mode.");

        let val = read_so_far.parse::<T>().unwrap();

        if let Some(hist) = &mut self.history {
            hist.write(&val);
        }

        Some(val)
    }
}

pub struct CLI;

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        s.chars().take(max_len - 3).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}

impl CLI {
    pub fn new() -> Self {
        if io::stdin().is_terminal() {}
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

    pub fn select<T: ToString + std::fmt::Debug>(
        prompt: &str,
        options: &[T],
        single: bool,
        selected: &[usize],
    ) -> Vec<usize> {
        terminal::enable_raw_mode().expect("Failed to set terminal to raw mode.");

        let mut selected_indices: Vec<usize> = selected.to_vec();
        let mut current_pos = selected.first().copied().unwrap_or(0);
        let mut query = String::new();
        let visible_count = 10.min(options.len());
        write!(std::io::stdout(), "{}\r", prompt).unwrap();

        for _ in 0..=visible_count {
            print!("\r\n");
        }

        let mut offset = current_pos.saturating_sub(visible_count - 1);
        let mut stdout = io::stdout();

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
            execute!(stdout, cursor::MoveUp(visible_count as u16)).unwrap();
        }

        fn get_filtered_options<T: ToString + std::fmt::Debug>(
            options_raw: &[T],
            query: &str,
        ) -> Vec<(usize, String)> {
            if query.is_empty() {
                options_raw
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i, v.to_string()))
                    .collect()
            } else {
                options_raw
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| {
                        fuzzy_match(&s.to_string(), query)
                            .filter(|&score| score > 0)
                            .map(|_| (i, s.to_string()))
                    })
                    .collect()
            }
        }

        fn draw(
            stdout: &mut io::Stdout,
            filtered_options: &[(usize, String)],
            current_pos: usize,
            selected_indices: &[usize],
            offset: usize,
            visible_count: usize,
            query: &str,
        ) {
            clear(stdout, visible_count);
            for j in offset..(offset + visible_count).min(filtered_options.len()) {
                execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine)).unwrap();
                let (orig_idx, ref option_str) = filtered_options[j];
                if j == current_pos {
                    print!("> ");
                } else {
                    print!("  ");
                }
                if selected_indices.contains(&orig_idx) {
                    print!("[x] ");
                } else {
                    print!("[ ] ");
                }
                let s = option_str
                    .replace("\n", "")
                    .replace("\r", "")
                    .replace("\t", " ");
                let s = truncate_string(&s, terminal::size().unwrap().0 as usize - 10);
                let s = strip_ansi_escapes::strip_str(s);
                write!(std::io::stdout(), "{}\r\n", s).unwrap();
            }
            if !query.is_empty() {
                execute!(io::stdout(), terminal::Clear(ClearType::CurrentLine)).unwrap();
                print!("\rQuery: {}\r", query);
            }
            stdout.flush().unwrap();
        }

        loop {
            let filtered_options = get_filtered_options(options, &query);
            if filtered_options.is_empty() {
                current_pos = 0;
                offset = 0;
            } else if current_pos >= filtered_options.len() {
                current_pos = filtered_options.len() - 1;
                offset = current_pos.saturating_sub(visible_count - 1);
            }

            draw(
                &mut stdout,
                &filtered_options,
                current_pos,
                &selected_indices,
                offset,
                visible_count,
                &query,
            );

            if event::poll(Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Up => {
                            if current_pos > 0 {
                                current_pos -= 1;
                                if current_pos < offset {
                                    offset = current_pos;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if current_pos < filtered_options.len().saturating_sub(1) {
                                current_pos += 1;
                                if current_pos >= offset + visible_count {
                                    offset = current_pos - visible_count + 1;
                                }
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some((orig_idx, _)) = filtered_options.get(current_pos) {
                                if single {
                                    selected_indices.clear();
                                    selected_indices.push(*orig_idx);
                                } else if selected_indices.contains(orig_idx) {
                                    selected_indices.retain(|&x| x != *orig_idx);
                                } else {
                                    selected_indices.push(*orig_idx);
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if single && selected_indices.is_empty() {
                                if let Some((orig_idx, _)) = filtered_options.get(current_pos) {
                                    selected_indices.push(*orig_idx);
                                }
                            }
                            break;
                        }
                        KeyCode::Esc => {
                            selected_indices.clear();
                            break;
                        }
                        KeyCode::Backspace => {
                            if !query.is_empty() {
                                query.pop();
                                current_pos = 0;
                            }
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                query.clear();
                                current_pos = 0;
                            }
                        }
                        KeyCode::Char(ch) => {
                            if ch == 'c' && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                break;
                            }
                            query.push(ch);
                            current_pos = 0;
                            offset = 0;
                            draw(
                                &mut stdout,
                                &filtered_options,
                                current_pos,
                                &selected_indices,
                                offset,
                                visible_count,
                                &query,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        for _ in 0..=visible_count {
            execute!(std::io::stdout(), cursor::MoveUp(1)).unwrap();
        }

        if !query.is_empty() {
            clear(&mut std::io::stdout(), visible_count + 2);
        } else {
            clear(&mut std::io::stdout(), visible_count + 1);
        }
        stdout.flush().unwrap();

        terminal::disable_raw_mode().expect("Failed to remove terminal to raw mode.");

        selected_indices.sort_unstable();
        selected_indices
    }
}
