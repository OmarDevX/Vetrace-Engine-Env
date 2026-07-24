use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

pub const HELP: &str = r#"Vetrace Player

Run a project-driven Vetrace game without game-specific Rust code.

USAGE:
    vetrace-player [OPTIONS] [PROJECT_OR_PACKAGE]

ARGS:
    <PROJECT_OR_PACKAGE>            Project directory, project.vetrace.toml, or .vpak
                                    [default: sidecar game.vpak, then current directory]

OPTIONS:
    -p, --project <PATH>            Project directory or manifest path
        --package <PATH>            Run a packaged .vpak project
        --headless                  Run without creating a game window
        --fixed-dt <SECONDS>        Override the project fixed timestep
        --main-scene <PROJECT_PATH> Override the configured main scene (project-relative)
        --max-frames <COUNT>        Stop after this many frames
        --debug-stdio               Enable Studio debugger protocol over stdin/stdout
        --validate-only             Validate the project, then exit
        --print-project-info        Print parsed project information, then exit
        --runtime-info              Print player/runtime capability metadata, then exit
        --write-template-metadata <PLAYER_BINARY>
                                    Write required export-template sidecar metadata
    -C, --working-directory <PATH>  Change directory before resolving PROJECT
    -h, --help                      Print help
    -V, --version                   Print version

EXIT STATUS:
    0  Success
    2  Command-line or unsupported-mode error
    3  Project load or validation error
    4  Runtime setup error
    5  Runtime execution error
"#;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerArgs {
    pub project: Option<PathBuf>,
    pub package: Option<PathBuf>,
    pub headless: bool,
    pub fixed_dt: Option<f32>,
    pub main_scene: Option<String>,
    pub max_frames: Option<usize>,
    pub debug_stdio: bool,
    pub validate_only: bool,
    pub print_project_info: bool,
    pub write_template_metadata: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
}

impl PlayerArgs {
    pub fn project_path(&self) -> PathBuf {
        self.project.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseOutcome {
    Run(PlayerArgs),
    Help,
    Version,
    RuntimeInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

pub fn parse_env() -> Result<ParseOutcome, CliError> {
    parse_from(std::env::args_os().skip(1))
}

pub fn parse_from<I, T>(args: I) -> Result<ParseOutcome, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into).peekable();
    let mut parsed = PlayerArgs::default();
    let mut positional_project: Option<PathBuf> = None;
    let mut parse_options = true;

    while let Some(argument) = args.next() {
        if parse_options && is_arg(&argument, "--") {
            parse_options = false;
            continue;
        }

        if parse_options {
            if is_arg(&argument, "-h") || is_arg(&argument, "--help") {
                return Ok(ParseOutcome::Help);
            }
            if is_arg(&argument, "-V") || is_arg(&argument, "--version") {
                return Ok(ParseOutcome::Version);
            }
            if is_arg(&argument, "--runtime-info") {
                return Ok(ParseOutcome::RuntimeInfo);
            }
            if is_arg(&argument, "--headless") {
                parsed.headless = true;
                continue;
            }
            if is_arg(&argument, "--debug-stdio") {
                parsed.debug_stdio = true;
                continue;
            }
            if is_arg(&argument, "--validate-only") {
                parsed.validate_only = true;
                continue;
            }
            if is_arg(&argument, "--print-project-info") {
                parsed.print_project_info = true;
                continue;
            }

            if let Some((name, value)) = split_long_option(&argument) {
                match name.as_str() {
                    "--project" => set_path(&mut parsed.project, "--project", value)?,
                    "--package" => set_path(&mut parsed.package, "--package", value)?,
                    "--fixed-dt" => {
                        set_fixed_dt(&mut parsed.fixed_dt, value)?;
                    }
                    "--main-scene" => {
                        set_string(&mut parsed.main_scene, "--main-scene", value)?;
                    }
                    "--max-frames" => {
                        set_max_frames(&mut parsed.max_frames, value)?;
                    }
                    "--working-directory" => {
                        set_path(&mut parsed.working_directory, "--working-directory", value)?;
                    }
                    "--write-template-metadata" => {
                        set_path(&mut parsed.write_template_metadata, "--write-template-metadata", value)?;
                    }
                    _ => return Err(CliError::new(format!("unknown option '{name}'"))),
                }
                continue;
            }

            if is_arg(&argument, "-p") || is_arg(&argument, "--project") {
                let value = next_value(&mut args, "--project")?;
                set_path(&mut parsed.project, "--project", value)?;
                continue;
            }
            if is_arg(&argument, "--package") {
                let value = next_value(&mut args, "--package")?;
                set_path(&mut parsed.package, "--package", value)?;
                continue;
            }
            if is_arg(&argument, "--fixed-dt") {
                let value = next_value(&mut args, "--fixed-dt")?;
                set_fixed_dt(&mut parsed.fixed_dt, value)?;
                continue;
            }
            if is_arg(&argument, "--main-scene") {
                let value = next_value(&mut args, "--main-scene")?;
                set_string(&mut parsed.main_scene, "--main-scene", value)?;
                continue;
            }
            if is_arg(&argument, "--max-frames") {
                let value = next_value(&mut args, "--max-frames")?;
                set_max_frames(&mut parsed.max_frames, value)?;
                continue;
            }
            if is_arg(&argument, "--write-template-metadata") {
                let value = next_value(&mut args, "--write-template-metadata")?;
                set_path(&mut parsed.write_template_metadata, "--write-template-metadata", value)?;
                continue;
            }
            if is_arg(&argument, "-C") || is_arg(&argument, "--working-directory") {
                let value = next_value(&mut args, "--working-directory")?;
                set_path(&mut parsed.working_directory, "--working-directory", value)?;
                continue;
            }

            if option_like(&argument) {
                return Err(CliError::new(format!(
                    "unknown option '{}'",
                    argument.to_string_lossy()
                )));
            }
        }

        if positional_project.replace(PathBuf::from(argument)).is_some() {
            return Err(CliError::new("only one positional PROJECT path is allowed"));
        }
    }

    if let Some(path) = positional_project {
        if parsed.project.is_some() || parsed.package.is_some() {
            return Err(CliError::new(
                "PROJECT_OR_PACKAGE cannot be supplied both positionally and with --project/--package",
            ));
        }
        if path.extension().and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vpak"))
        {
            parsed.package = Some(path);
        } else {
            parsed.project = Some(path);
        }
    }

    if parsed.project.is_some() && parsed.package.is_some() {
        return Err(CliError::new(
            "--project and --package are mutually exclusive",
        ));
    }

    Ok(ParseOutcome::Run(parsed))
}

fn is_arg(argument: &OsString, expected: &str) -> bool {
    argument.as_os_str() == OsStr::new(expected)
}

fn split_long_option(argument: &OsString) -> Option<(String, OsString)> {
    let text = argument.to_str()?;
    if !text.starts_with("--") {
        return None;
    }
    let (name, value) = text.split_once('=')?;
    Some((name.to_owned(), OsString::from(value)))
}

fn next_value<I>(args: &mut std::iter::Peekable<I>, option: &str) -> Result<OsString, CliError>
where
    I: Iterator<Item = OsString>,
{
    args.next().ok_or_else(|| CliError::new(format!("{option} requires a value")))
}

fn set_path(slot: &mut Option<PathBuf>, option: &str, value: OsString) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new(format!("{option} may only be specified once")));
    }
    if value.as_os_str().is_empty() {
        return Err(CliError::new(format!("{option} cannot be empty")));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_string(slot: &mut Option<String>, option: &str, value: OsString) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new(format!("{option} may only be specified once")));
    }
    let value = parse_utf8(value, option)?;
    if value.is_empty() {
        return Err(CliError::new(format!("{option} cannot be empty")));
    }
    *slot = Some(value);
    Ok(())
}

fn set_fixed_dt(slot: &mut Option<f32>, value: OsString) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new("--fixed-dt may only be specified once"));
    }
    let value = parse_utf8(value, "--fixed-dt")?;
    let parsed: f32 = value
        .parse()
        .map_err(|_| CliError::new(format!("invalid --fixed-dt value '{value}'")))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(CliError::new(
            "--fixed-dt must be a finite number greater than zero",
        ));
    }
    *slot = Some(parsed);
    Ok(())
}

fn set_max_frames(slot: &mut Option<usize>, value: OsString) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new("--max-frames may only be specified once"));
    }
    let value = parse_utf8(value, "--max-frames")?;
    let parsed: usize = value
        .parse()
        .map_err(|_| CliError::new(format!("invalid --max-frames value '{value}'")))?;
    *slot = Some(parsed);
    Ok(())
}

fn parse_utf8(value: OsString, option: &str) -> Result<String, CliError> {
    value
        .into_string()
        .map_err(|_| CliError::new(format!("{option} requires a UTF-8 value")))
}

fn option_like(argument: &OsString) -> bool {
    argument.to_string_lossy().starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_positional_project() {
        let ParseOutcome::Run(args) = parse_from(["game"]).unwrap() else { panic!() };
        assert_eq!(args.project, Some(PathBuf::from("game")));
    }


    #[test]
    fn positional_vpak_is_treated_as_a_package() {
        let ParseOutcome::Run(args) = parse_from(["game.vpak"]).unwrap() else { panic!() };
        assert_eq!(args.package, Some(PathBuf::from("game.vpak")));
        assert!(args.project.is_none());
    }

    #[test]
    fn accepts_long_and_equals_options() {
        let ParseOutcome::Run(args) = parse_from([
            "--project=game",
            "--headless",
            "--fixed-dt=0.02",
            "--max-frames",
            "5",
        ])
        .unwrap() else { panic!() };
        assert!(args.headless);
        assert_eq!(args.fixed_dt, Some(0.02));
        assert_eq!(args.max_frames, Some(5));
    }

    #[test]
    fn supports_non_utf8_paths_on_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let path = OsString::from_vec(vec![b'g', b'a', b'm', b'e', 0xff]);
            let ParseOutcome::Run(args) = parse_from([path.clone()]).unwrap() else { panic!() };
            assert_eq!(args.project, Some(PathBuf::from(path)));
        }
    }

    #[test]
    fn rejects_duplicate_project_sources() {
        let error = parse_from(["--project", "game", "other"]).unwrap_err();
        assert!(error.to_string().contains("both positionally"));
    }

    #[test]
    fn rejects_project_and_package() {
        let error = parse_from(["--project", "game", "--package", "game.vpak"]).unwrap_err();
        assert!(error.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_invalid_delta() {
        assert!(parse_from(["--fixed-dt", "0"]).is_err());
        assert!(parse_from(["--fixed-dt", "NaN"]).is_err());
    }

    #[test]
    fn double_dash_allows_dash_prefixed_path() {
        let ParseOutcome::Run(args) = parse_from(["--", "-game"]).unwrap() else { panic!() };
        assert_eq!(args.project, Some(PathBuf::from("-game")));
    }

    #[test]
    fn defaults_to_current_directory() {
        let ParseOutcome::Run(args) = parse_from(std::iter::empty::<&str>()).unwrap() else { panic!() };
        assert_eq!(args.project_path(), PathBuf::from("."));
    }

    #[test]
    fn help_and_version_short_circuit() {
        assert_eq!(parse_from(["--help"]).unwrap(), ParseOutcome::Help);
        assert_eq!(parse_from(["-V"]).unwrap(), ParseOutcome::Version);
        assert_eq!(parse_from(["--runtime-info"]).unwrap(), ParseOutcome::RuntimeInfo);
    }
}
