use bat::PrettyPrinter;
use std::io::{Error, IsTerminal, self};
use std::pin::Pin;
use tokio_stream::StreamExt;

pub async fn process_response(
    stream: Pin<Box<dyn tokio_stream::Stream<Item = Result<String, Error>>>>,
    code_blocks: &mut Vec<String>,
) -> Result<String, Error> {
    tokio::pin!(stream);

    let mut in_code_block = false;
    let mut language = String::new();
    let mut full_response = String::new();
    let mut current_code_block_content = String::new();
    let mut tickcnt = 0;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(content) => {
                if !io::stdout().is_terminal() {
                    print!("{}", content);
                } else {
                    for ch in content.chars() {
                        if ch == '`' {
                            tickcnt += 1;
                        }
                    }

                    full_response.push_str(&content);
                    if !in_code_block {
                        if tickcnt == 3 {
                            tickcnt = 0;
                            in_code_block = true;
                            language = String::new();
                        } else {
                            print!("{}", content);
                        }
                    } else {
                        if tickcnt == 3 {
                            tickcnt = 0;
                            in_code_block = false;

                            code_blocks.push(current_code_block_content.clone());
                            let block = current_code_block_content.clone();
                            let l = language.clone();

                            let mut pp = PrettyPrinter::new();
                            pp.input_from_bytes(block.as_bytes()).colored_output(true);
                            if language != " " {
                                pp.language(&l);
                            }

                            pp.print().unwrap();

                            language = String::new();

                            if content.ends_with('\n') {
                                println!();
                            }

                            current_code_block_content = String::new();
                        } else if language == "" {
                            if content == "\n" {
                                language = " ".to_string();
                            } else {
                                language = content.trim().to_string();
                            }
                        } else {
                            current_code_block_content.push_str(&content);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }
    Ok(full_response)
}
