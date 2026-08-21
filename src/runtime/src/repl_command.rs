//! Shared, typed command language for every interactive Mech host.

use crate::{MAX_RESIDENT_STEP_COUNT, validate_resident_step_count};

pub const ARGUMENT_QUOTING_HELP: &str = "Command arguments containing spaces may be wrapped in \
single or double quotes. Backslashes are literal; repeat the quote delimiter to include it inside \
a quoted argument.";

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplCommandId {
    Help,
    Version,
    Docs,
    Capabilities,
    Whos,
    Constraints,
    Plan,
    Outputs,
    Output,
    Step,
    Load,
    Save,
    Code,
    List,
    ChangeDirectory,
    Clear,
    ClearInteraction,
    Profile,
    Quit,
}

impl ReplCommandId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Version => "version",
            Self::Docs => "docs",
            Self::Capabilities => "capabilities",
            Self::Whos => "whos",
            Self::Constraints => "constraints",
            Self::Plan => "plan",
            Self::Outputs => "outputs",
            Self::Output => "output",
            Self::Step => "step",
            Self::Load => "load",
            Self::Save => "save",
            Self::Code => "code",
            Self::List => "ls",
            Self::ChangeDirectory => "cd",
            Self::Clear => "clear",
            Self::ClearInteraction => "clc",
            Self::Profile => "profile",
            Self::Quit => "quit",
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "data", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplRequest {
    SubmitSource {
        source: String,
        origin: SourceOrigin,
    },
    InvokeCommand {
        command: ReplCommand,
        source: String,
    },
    Complete {
        input: String,
        cursor: usize,
    },
    Interrupt,
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceOrigin {
    Interactive,
    Command,
    Resource(String),
    Host(String),
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(
    feature = "serde",
    serde(tag = "command", content = "arguments", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplCommand {
    Capabilities,
    Cd(String),
    Clc,
    Clear(Vec<String>),
    Code(String),
    Constraints(Vec<String>),
    Docs(Option<String>),
    Help,
    Version,
    Load(Vec<String>),
    Ls(Option<String>),
    Output(String),
    Outputs,
    Plan,
    Profile(Option<bool>),
    Quit,
    Save(String),
    Step { selector: Option<usize>, count: u64 },
    Whos(Vec<String>),
}

impl ReplCommand {
    pub const fn id(&self) -> ReplCommandId {
        match self {
            Self::Capabilities => ReplCommandId::Capabilities,
            Self::Cd(_) => ReplCommandId::ChangeDirectory,
            Self::Clc => ReplCommandId::ClearInteraction,
            Self::Clear(_) => ReplCommandId::Clear,
            Self::Code(_) => ReplCommandId::Code,
            Self::Constraints(_) => ReplCommandId::Constraints,
            Self::Docs(_) => ReplCommandId::Docs,
            Self::Help => ReplCommandId::Help,
            Self::Version => ReplCommandId::Version,
            Self::Load(_) => ReplCommandId::Load,
            Self::Ls(_) => ReplCommandId::List,
            Self::Output(_) => ReplCommandId::Output,
            Self::Outputs => ReplCommandId::Outputs,
            Self::Plan => ReplCommandId::Plan,
            Self::Profile(_) => ReplCommandId::Profile,
            Self::Quit => ReplCommandId::Quit,
            Self::Save(_) => ReplCommandId::Save,
            Self::Step { .. } => ReplCommandId::Step,
            Self::Whos(_) => ReplCommandId::Whos,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplHostRequirement {
    Portable,
    Documentation,
    ReadableResources,
    WritableResources,
    WorkingDirectory,
    InteractionControl,
    Profiling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplCommandSpec {
    pub id: ReplCommandId,
    pub usage: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub requirement: ReplHostRequirement,
}

pub const REPL_COMMAND_SPECS: &[ReplCommandSpec] = &[
    spec(
        ReplCommandId::Help,
        ":help",
        "show this command index",
        &["h", "?"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Version,
        ":version",
        "show installed Mech, library, and host versions",
        &["v"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Docs,
        ":docs [topic]",
        "list or search embedded documentation",
        &["d"],
        ReplHostRequirement::Documentation,
    ),
    spec(
        ReplCommandId::Capabilities,
        ":capabilities",
        "show effective REPL host grants",
        &["capability", "caps"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Whos,
        ":whos [names...]",
        "show resident symbol types and values",
        &["w"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Constraints,
        ":constraints [names...]",
        "show integrity constraint types and values",
        &[],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Plan,
        ":plan",
        "show the active resident plan summary",
        &["p"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Outputs,
        ":outputs",
        "list output artifacts in this session",
        &[],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Output,
        ":output <id>",
        "inspect or focus one output artifact",
        &[],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Step,
        ":step [count]",
        "step the entire active resident program",
        &[],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::Load,
        ":load <paths...>",
        "transactionally append source resources",
        &[],
        ReplHostRequirement::ReadableResources,
    ),
    spec(
        ReplCommandId::Save,
        ":save <path>",
        "save accepted session source",
        &[],
        ReplHostRequirement::WritableResources,
    ),
    spec(
        ReplCommandId::Code,
        ":code <source>",
        "evaluate source from one command line",
        &["c"],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::List,
        ":ls [path]",
        "list a resource collection",
        &[],
        ReplHostRequirement::ReadableResources,
    ),
    spec(
        ReplCommandId::ChangeDirectory,
        ":cd <path>",
        "change the working resource collection",
        &[],
        ReplHostRequirement::WorkingDirectory,
    ),
    spec(
        ReplCommandId::Clear,
        ":clear [names...]",
        "remove resident variables; no names clears the workspace",
        &[],
        ReplHostRequirement::Portable,
    ),
    spec(
        ReplCommandId::ClearInteraction,
        ":clc",
        "clear the interaction transcript",
        &[],
        ReplHostRequirement::InteractionControl,
    ),
    spec(
        ReplCommandId::Profile,
        ":profile [on|off]",
        "report resident profiling availability",
        &[],
        ReplHostRequirement::Profiling,
    ),
    spec(
        ReplCommandId::Quit,
        ":quit",
        "terminate this REPL session",
        &["exit", "q"],
        ReplHostRequirement::InteractionControl,
    ),
];

const fn spec(
    id: ReplCommandId,
    usage: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    requirement: ReplHostRequirement,
) -> ReplCommandSpec {
    ReplCommandSpec {
        id,
        usage,
        description,
        aliases,
        requirement,
    }
}

pub fn parse_repl_request(input: &str) -> Result<ReplRequest, String> {
    if input.trim_start().starts_with(':') {
        parse_repl_command(input).map(|command| ReplRequest::InvokeCommand {
            command,
            source: input.to_string(),
        })
    } else {
        Ok(ReplRequest::SubmitSource {
            source: input.to_string(),
            origin: SourceOrigin::Interactive,
        })
    }
}

pub fn parse_repl_command(input: &str) -> Result<ReplCommand, String> {
    let command = input.trim();
    let body = command
        .strip_prefix(':')
        .ok_or_else(|| "REPL commands must start with `:`.".to_string())?;
    let (name, arguments) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    let name = name.to_ascii_lowercase();
    let arguments = arguments.trim();

    match name.as_str() {
        "help" | "h" | "?" => no_arguments(arguments, ":help", ReplCommand::Help),
        "version" | "v" => no_arguments(arguments, ":version", ReplCommand::Version),
        "quit" | "exit" | "q" => no_arguments(arguments, ":quit", ReplCommand::Quit),
        "clear" => Ok(ReplCommand::Clear(split_arguments(arguments)?)),
        "clc" => no_arguments(arguments, ":clc", ReplCommand::Clc),
        "capabilities" | "capability" | "caps" => {
            no_arguments(arguments, ":capabilities", ReplCommand::Capabilities)
        }
        "plan" | "p" => no_arguments(arguments, ":plan", ReplCommand::Plan),
        "outputs" => no_arguments(arguments, ":outputs", ReplCommand::Outputs),
        "output" => Ok(ReplCommand::Output(single_argument(
            arguments,
            ":output <id>",
        )?)),
        "docs" | "d" => {
            let values = split_arguments(arguments)?;
            Ok(ReplCommand::Docs(
                (!values.is_empty()).then(|| values.join(" ")),
            ))
        }
        "whos" | "w" => Ok(ReplCommand::Whos(split_arguments(arguments)?)),
        "constraints" => Ok(ReplCommand::Constraints(split_arguments(arguments)?)),
        "load" => {
            let values = split_arguments(arguments)?;
            if values.is_empty() {
                return Err("Usage: :load <paths...>".to_string());
            }
            Ok(ReplCommand::Load(values))
        }
        "save" => Ok(ReplCommand::Save(single_argument(
            arguments,
            ":save <path>",
        )?)),
        "cd" => Ok(ReplCommand::Cd(single_argument(arguments, ":cd <path>")?)),
        "ls" => {
            let values = split_arguments(arguments)?;
            if values.len() > 1 {
                return Err("Usage: :ls [path]".to_string());
            }
            Ok(ReplCommand::Ls(values.into_iter().next()))
        }
        "code" | "c" => {
            if arguments.is_empty() {
                return Err("Usage: :code <source>".to_string());
            }
            Ok(ReplCommand::Code(arguments.to_string()))
        }
        "profile" => parse_profile(arguments),
        "step" => parse_step(arguments),
        "" => Err("Enter `:help` for the command index.".to_string()),
        _ => Err(format!(
            "Unknown command `:{name}`. Enter `:help` for the command index."
        )),
    }
}

fn no_arguments(arguments: &str, usage: &str, command: ReplCommand) -> Result<ReplCommand, String> {
    if arguments.is_empty() {
        Ok(command)
    } else {
        Err(format!("Usage: {usage}"))
    }
}

fn single_argument(arguments: &str, usage: &str) -> Result<String, String> {
    let values = split_arguments(arguments)?;
    if values.len() == 1 {
        Ok(values.into_iter().next().expect("one argument"))
    } else {
        Err(format!("Usage: {usage}"))
    }
}

pub fn split_repl_arguments(input: &str) -> Result<Vec<String>, String> {
    split_arguments(input)
}

fn split_arguments(input: &str) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut token_started = false;
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        match quote {
            Some(delimiter) if character == delimiter => {
                if characters.peek().copied() == Some(delimiter) {
                    current.push(characters.next().expect("peeked quote delimiter"));
                } else {
                    quote = None;
                }
                token_started = true;
            }
            Some(_) => {
                current.push(character);
                token_started = true;
            }
            None if matches!(character, '\'' | '"') => {
                quote = Some(character);
                token_started = true;
            }
            None if character.is_whitespace() => {
                if token_started {
                    arguments.push(core::mem::take(&mut current));
                    token_started = false;
                }
            }
            None => {
                current.push(character);
                token_started = true;
            }
        }
    }
    if let Some(delimiter) = quote {
        return Err(format!("Unclosed {delimiter} quote in command arguments."));
    }
    if token_started {
        arguments.push(current);
    }
    Ok(arguments)
}

fn parse_profile(arguments: &str) -> Result<ReplCommand, String> {
    let values = split_arguments(arguments)?;
    match values.as_slice() {
        [] => Ok(ReplCommand::Profile(None)),
        [value] if value == "on" => Ok(ReplCommand::Profile(Some(true))),
        [value] if value == "off" => Ok(ReplCommand::Profile(Some(false))),
        _ => Err("Usage: :profile [on|off]".to_string()),
    }
}

fn parse_step(arguments: &str) -> Result<ReplCommand, String> {
    let pieces = split_arguments(arguments)?;
    let (selector, count) = match pieces.as_slice() {
        [] => (None, 1),
        [count] if !count.starts_with('#') => (None, parse_step_count(count)?),
        [selector] if selector.starts_with('#') => (Some(parse_step_selector(selector)?), 1),
        [selector, count] if selector.starts_with('#') => (
            Some(parse_step_selector(selector)?),
            parse_step_count(count)?,
        ),
        _ => return Err("Usage: :step [count]".to_string()),
    };
    Ok(ReplCommand::Step { selector, count })
}

fn parse_step_selector(value: &str) -> Result<usize, String> {
    value
        .strip_prefix('#')
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("Invalid step selector `{value}`."))
}

fn parse_step_count(value: &str) -> Result<u64, String> {
    let count = value
        .parse::<u64>()
        .map_err(|_| format!("Invalid step count `{value}`."))?;
    validate_resident_step_count(count)
        .map_err(|_| format!("step count must be between 1 and {MAX_RESIDENT_STEP_COUNT}."))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique_and_parser_covers_every_command() {
        let ids = REPL_COMMAND_SPECS
            .iter()
            .map(|spec| spec.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), REPL_COMMAND_SPECS.len());
        for spec in REPL_COMMAND_SPECS {
            let minimal = match spec.id {
                ReplCommandId::Output => ":output item",
                ReplCommandId::Load => ":load file.mec",
                ReplCommandId::Save => ":save file.mec",
                ReplCommandId::Code => ":code 1 + 1",
                ReplCommandId::ChangeDirectory => ":cd path",
                _ => spec
                    .usage
                    .split_once(' ')
                    .map_or(spec.usage, |(name, _)| name),
            };
            let parsed = parse_repl_command(minimal).unwrap();
            assert_eq!(parsed.id(), spec.id, "registry entry {}", spec.usage);
        }
    }

    #[test]
    fn parser_preserves_literal_windows_and_unc_separators() {
        assert_eq!(
            parse_repl_command(r":cd C:\work\mech"),
            Ok(ReplCommand::Cd(r"C:\work\mech".to_string()))
        );
        assert_eq!(
            parse_repl_command(r#":load "\\server\shared folder\one.mec" "C:\work\two file.mec""#),
            Ok(ReplCommand::Load(vec![
                r"\\server\shared folder\one.mec".to_string(),
                r"C:\work\two file.mec".to_string()
            ]))
        );
    }

    #[test]
    fn request_parser_is_the_single_source_and_command_boundary() {
        assert!(matches!(
            parse_repl_request("x := 1\n"),
            Ok(ReplRequest::SubmitSource { .. })
        ));
        assert_eq!(
            parse_repl_request(":caps"),
            Ok(ReplRequest::InvokeCommand {
                command: ReplCommand::Capabilities,
                source: ":caps".to_string(),
            })
        );
        assert!(parse_repl_command(":step 0").is_err());
        assert!(parse_repl_command(":step 1000001").is_err());
        assert!(parse_repl_command(":symbols").is_err());
        assert!(parse_repl_command(":s").is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn typed_requests_round_trip_across_the_serialized_embedding_boundary() {
        let request = ReplRequest::InvokeCommand {
            command: ReplCommand::Load(vec!["models/odd name.mec".to_string()]),
            source: ":load \"models/odd name.mec\"".to_string(),
        };
        let encoded = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serde_json::from_str::<ReplRequest>(&encoded).unwrap(),
            request
        );
    }
}
