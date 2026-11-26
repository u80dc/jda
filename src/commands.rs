#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Info,
    Exit,
}

pub(crate) struct CommandDefinition {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) command: Command,
}

pub(crate) const COMMANDS: &[CommandDefinition] = &[
    CommandDefinition {
        name: "/help",
        description: "Show help",
        command: Command::Help,
    },
    CommandDefinition {
        name: "/info",
        description: "Show file information",
        command: Command::Info,
    },
    CommandDefinition {
        name: "/q",
        description: "Quit",
        command: Command::Exit,
    },
    CommandDefinition {
        name: "/exit",
        description: "Quit (alias of /q)",
        command: Command::Exit,
    },
];

pub(crate) fn parse(input: &str) -> Option<Command> {
    let name = input.trim();
    COMMANDS
        .iter()
        .find(|definition| definition.name.eq_ignore_ascii_case(name))
        .map(|definition| definition.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_aliases_ignore_case_and_surrounding_whitespace() {
        for input in [
            "/q",
            "/Q",
            "/exit",
            "/EXIT",
            "/ExIt",
            " \t/exit\r\n",
            " /Q ",
        ] {
            assert_eq!(parse(input), Some(Command::Exit), "input: {input:?}");
        }
    }

    #[test]
    fn incomplete_or_extended_exit_commands_are_not_executed() {
        for input in ["", "/", "/e", "/exit now", "/exitnow", r"\exit", "exit"] {
            assert_eq!(parse(input), None, "input: {input:?}");
        }
        assert_eq!(parse("/help"), Some(Command::Help));
        assert_eq!(parse("/info"), Some(Command::Info));
    }
}
