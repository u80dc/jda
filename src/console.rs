use std::path::Path;

use anyhow::Result;
use rustyline::{
    Cmd, Config, Context, Editor, EventHandler, Helper, KeyCode, KeyEvent, Modifiers,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::{Hint, Hinter},
    history::DefaultHistory,
    validate::Validator,
};

const COMMANDS: &[&str] = &["/help", "/info", "/q"];

struct Commands;

impl Helper for Commands {}
impl Highlighter for Commands {}
impl Validator for Commands {}
struct CommandHint {
    display: String,
    completion: Option<String>,
}

impl Hint for CommandHint {
    fn display(&self) -> &str {
        &self.display
    }

    fn completion(&self) -> Option<&str> {
        self.completion.as_deref()
    }
}

impl Hinter for Commands {
    type Hint = CommandHint;

    fn hint(&self, line: &str, pos: usize, _context: &Context<'_>) -> Option<CommandHint> {
        if pos != line.len() {
            return None;
        }
        let prefix = line.trim_start().to_ascii_lowercase();
        if prefix.is_empty() || prefix.contains(char::is_whitespace) {
            return None;
        }
        let matches: Vec<_> = COMMANDS
            .iter()
            .copied()
            .filter(|command| command.starts_with(&prefix))
            .collect();
        match matches.as_slice() {
            [command] if command.len() > prefix.len() => {
                let suffix = command[prefix.len()..].to_owned();
                Some(CommandHint {
                    display: suffix.clone(),
                    completion: Some(suffix),
                })
            }
            [_, _, ..] => Some(CommandHint {
                display: format!("  [{}]", matches.join("  ")),
                completion: None,
            }),
            _ => None,
        }
    }
}

impl Completer for Commands {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        let start = prefix.len() - prefix.trim_start().len();
        let word = &prefix[start..];
        if word.contains(char::is_whitespace) {
            return Ok((pos, Vec::new()));
        }
        let candidates = COMMANDS
            .iter()
            .filter(|command| command.starts_with(&word.to_ascii_lowercase()))
            .map(|command| Pair {
                display: (*command).to_owned(),
                replacement: (*command).to_owned(),
            })
            .collect();
        Ok((start, candidates))
    }
}

pub fn run(path: &Path, size: u64) -> Result<()> {
    let config = Config::builder().max_history_size(100)?.build();
    let mut editor = Editor::<Commands, DefaultHistory>::with_config(config)?;
    editor.set_helper(Some(Commands));
    editor.bind_sequence(
        KeyEvent(KeyCode::Esc, Modifiers::NONE),
        EventHandler::Simple(Cmd::Interrupt),
    );

    println!("jda · Java Heap Dump Analyzer\n");
    print_info(path, size);
    println!();

    loop {
        match editor.readline("jda> ") {
            Ok(line) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }
                if command.eq_ignore_ascii_case("/q") {
                    break;
                }
                // Avoid retaining large pasted input in history.
                if command.len() > 256 {
                    println!("Command is too long (maximum 256 bytes).\n");
                    continue;
                }
                editor.add_history_entry(command)?;
                if command.eq_ignore_ascii_case("/help") {
                    println!(
                        r"Commands:
  /help  Show help
  /info  Show file information"
                    );
                    println!(r"  /q     Quit (also /Q)");
                    println!("\nLeft/Right: move cursor | Up/Down: history | Tab: complete");
                    println!("Type a command prefix for hints; Right arrow accepts a unique hint.");
                    println!("Esc / Ctrl+C: quit | Ctrl+D: quit on empty input");
                    println!("Heap analysis is not implemented yet.\n");
                } else if command.eq_ignore_ascii_case("/info") {
                    print_info(path, size);
                    println!();
                } else {
                    println!("Unknown command. Type /help for commands.\n");
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(error) => return Err(error.into()),
        }
    }
    println!("Bye.");
    Ok(())
}

fn print_info(path: &Path, size: u64) {
    println!("File  {}", path.display());
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    println!("Size  {value:.2} {} ({size} bytes)", units[unit]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_list_is_display_only() {
        let history = DefaultHistory::new();
        let context = Context::new(&history);
        let hint = Commands.hint(r"/", 1, &context).unwrap();
        assert_eq!(hint.display(), r"  [/help  /info  /q]");
        assert_eq!(hint.completion(), None);
    }

    #[test]
    fn unique_prefix_completes_case_insensitively() {
        let history = DefaultHistory::new();
        let context = Context::new(&history);
        for prefix in [r"/h", r"/H"] {
            let hint = Commands.hint(prefix, prefix.len(), &context).unwrap();
            assert_eq!(hint.display(), "elp");
            assert_eq!(hint.completion(), Some("elp"));
            let (_, candidates) = Commands.complete(prefix, prefix.len(), &context).unwrap();
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].replacement, r"/help");
        }
    }

    #[test]
    fn hints_do_not_appear_for_invalid_or_completed_input() {
        let history = DefaultHistory::new();
        let context = Context::new(&history);
        for line in ["", r"/help", r"/unknown", r"/help argument", "中文"] {
            assert!(Commands.hint(line, line.len(), &context).is_none());
        }
        assert!(Commands.hint(r"/help", 2, &context).is_none());
    }
}
