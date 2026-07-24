use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct StudioArgs {
    pub project: Option<PathBuf>,
    pub project_manager: bool,
    pub max_frames: Option<usize>,
}

impl StudioArgs {
    pub fn parse() -> Result<Self, String> {
        Self::parse_from(env::args_os().skip(1))
    }

    fn parse_from<I, S>(arguments: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut project = None;
        let mut project_manager = false;
        let mut max_frames = None;
        let mut args = arguments.into_iter().map(Into::into);
        while let Some(argument) = args.next() {
            let arg = argument.to_string_lossy().into_owned();
            match arg.as_str() {
                "--project" => {
                    project = Some(PathBuf::from(
                        args.next().ok_or("--project requires a path")?,
                    ));
                }
                "--project-manager" => project_manager = true,
                "--max-frames" => {
                    let value = args.next().ok_or("--max-frames requires a number")?;
                    let value = value.to_string_lossy();
                    max_frames = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("invalid --max-frames value `{value}`"))?,
                    );
                }
                "-h" | "--help" => return Err(Self::usage()),
                value if value.starts_with('-') => {
                    return Err(format!("unknown argument `{value}`\n\n{}", Self::usage()));
                }
                _ => {
                    if project.is_some() {
                        return Err(format!("multiple project paths provided\n\n{}", Self::usage()));
                    }
                    project = Some(PathBuf::from(argument));
                }
            }
        }
        if project_manager && project.is_some() {
            return Err("--project-manager cannot be combined with a project path".to_string());
        }
        Ok(Self { project, project_manager, max_frames })
    }

    pub fn usage() -> String {
        "Usage: vetrace-studio [PROJECT_DIRECTORY] [--max-frames N]\n       vetrace-studio --project <PROJECT_DIRECTORY>\n       vetrace-studio --project-manager"
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_opens_project_manager() {
        let args = StudioArgs::parse_from(Vec::<String>::new()).unwrap();
        assert!(args.project.is_none());
        assert!(!args.project_manager);
    }

    #[test]
    fn positional_project_remains_supported() {
        let args = StudioArgs::parse_from(["examples/game"]).unwrap();
        assert_eq!(args.project, Some(PathBuf::from("examples/game")));
    }
}
