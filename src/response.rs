use bat::PrettyPrinter;
use std::io::IsTerminal;
use std::pin::Pin;
use tokio::io::{self, AsyncWriteExt, Error};
use tokio_stream::StreamExt;

pub async fn process_response(
    stream: Pin<Box<dyn tokio_stream::Stream<Item = Result<String, Error>>>>,
    code_blocks: &mut Vec<String>,
    raw: bool,
) -> Result<String, Error> {
    tokio::pin!(stream);

    let mut in_code_block = false;
    let mut language_reading = false;
    let mut language = String::new();
    let mut full_response = String::new();
    let mut current_code_block_content = String::new();
    let mut tick_count = 0;
    let mut star_cnt = 0;
    let mut in_effect = false;
    let mut text_effected = false;
    let mut next_newline_reset = true;
    let stdout_is_terminal = std::io::stdout().is_terminal();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(content) => {
                if raw {
                    print!("{}", content);
                } else {
                    let mut chars = content.chars().peekable();

                    while let Some(ch) = chars.next() {
                        if ch == '\n' && next_newline_reset {
                            print!("\x1b[0m");
                        }

                        if language_reading {
                            if ch == '\n' {
                                language_reading = false;
                            } else {
                                language.push(ch);
                                in_code_block = true;
                            }
                        } else if ch == '`' {
                            tick_count += 1;
                            if tick_count == 3 {
                                tick_count = 0;

                                if in_code_block {
                                    in_code_block = false;
                                    code_blocks.push(current_code_block_content.clone());

                                    if stdout_is_terminal {
                                        let mut language = language.trim().to_owned();
                                        if language == "csharp" {
                                            language = "c#".to_owned();
                                        } else if language == "fsharp" {
                                            language = "f#".to_owned();
                                        }

                                        let mut pp = PrettyPrinter::new();
                                        pp.input_from_bytes(current_code_block_content.as_bytes())
                                            .colored_output(true);

                                        if !language.is_empty() {
                                            pp.language(&language);
                                        }

                                        pp.print().unwrap();
                                    } else {
                                        println!("{}", current_code_block_content);
                                    }

                                    current_code_block_content.clear();
                                    language.clear();
                                } else {
                                    in_code_block = true;
                                    language_reading = true;
                                    language.clear();
                                }
                            }
                        } else if !in_code_block && (ch == '*' || ch == '_') {
                            if text_effected {
                                star_cnt -= 1;
                                if star_cnt == 0 {
                                    in_effect = false;
                                    print!("\x1b[0m");
                                    text_effected = false;
                                }
                            } else {
                                star_cnt += 1;
                                in_effect = true;
                                if star_cnt == 1 {
                                    print!("\x1b[0;3m");
                                } else if star_cnt == 2 {
                                    print!("\x1b[0;1m");
                                } else if star_cnt == 3 {
                                    print!("\x1b[0;1;3m");
                                }
                            }
                        } else if !in_code_block && ch == '#' {
                            print!("\x1b[1m#");
                            next_newline_reset = true;
                        } else {
                            if in_effect {
                                text_effected = true;
                            }

                            if tick_count > 0 {
                                full_response.push_str(&"`".repeat(tick_count));
                                if stdout_is_terminal {
                                    print!("{}", "`".repeat(tick_count));
                                    io::stdout().flush().await.unwrap();
                                }
                                tick_count = 0;
                            }

                            if in_code_block {
                                if language.is_empty() {
                                    if ch == '\n' {
                                        language = " ".to_string();
                                    } else {
                                        language.push(ch);
                                    }
                                } else {
                                    current_code_block_content.push(ch);
                                }
                            } else {
                                full_response.push(ch);
                                if stdout_is_terminal {
                                    print!("{}", ch);
                                    io::stdout().flush().await.unwrap();
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                eprint!("Error: {}\r\n", err);
                return Err(err);
            }
        }
    }

    Ok(full_response)
}
