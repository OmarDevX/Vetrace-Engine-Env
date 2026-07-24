use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use vetrace_project::ProjectPath;
use vetrace_scripting_lua::{LuaDebuggerCommand, LuaDebuggerEvent};

const DEBUG_COMMAND_PREFIX: &str = "VETRACE_DEBUG_COMMAND\t";
const DEBUG_EVENT_PREFIX: &str = "VETRACE_DEBUG_EVENT\t";
const REQUIRED_PLAYER_CAPABILITIES: &[&str] = &["screen_ui_text_v1", "lua_project_modules_v1"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerOutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerOutputLine {
    pub stream: PlayerOutputStream,
    pub text: String,
}


#[derive(Clone, Debug, PartialEq)]
pub enum PlayerProcessEvent {
    Output(PlayerOutputLine),
    Debugger(LuaDebuggerEvent),
}

pub struct PlayerProcess {
    child: Option<Child>,
    events: Option<Receiver<PlayerProcessEvent>>,
    stdin: Option<ChildStdin>,
    last_exit: Option<ExitStatus>,
    cleanup_file: Option<PathBuf>,
}

impl PlayerProcess {
    pub fn new() -> Self {
        Self { child: None, events: None, stdin: None, last_exit: None, cleanup_file: None }
    }

    pub fn is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else { return false; };
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                self.last_exit = Some(status);
                self.child = None;
                self.stdin = None;
                self.cleanup_play_file();
                false
            }
            Err(_) => {
                self.child = None;
                self.stdin = None;
                self.cleanup_play_file();
                false
            }
        }
    }

    pub fn start(
        &mut self,
        project_root: &Path,
        main_scene_override: Option<&ProjectPath>,
        cleanup_file: Option<PathBuf>,
        debug: bool,
    ) -> Result<(), String> {
        if self.is_running() {
            return Ok(());
        }
        let executable = resolve_player_executable();
        let mut command = if let Some(executable) = executable {
            let mut command = Command::new(executable);
            command.arg(project_root).current_dir(project_root);
            command
        } else if let Some(workspace) = resolve_development_workspace() {
            // Development builds must not silently run an older sibling player.
            // Cargo rebuilds the current player when the capability handshake is
            // missing or incompatible.
            let mut command = Command::new("cargo");
            command
                .arg("run")
                .arg("-p")
                .arg("vetrace_player")
                .arg("--")
                .arg(project_root)
                .current_dir(workspace);
            command
        } else {
            return Err(format!(
                "no compatible vetrace-player was found. The player must advertise these runtime capabilities: {}. Rebuild/install the matching player or set VETRACE_PLAYER to it",
                REQUIRED_PLAYER_CAPABILITIES.join(", "),
            ));
        };
        if let Some(main_scene) = main_scene_override {
            command.arg("--main-scene").arg(main_scene.as_str());
        }
        if debug { command.arg("--debug-stdio"); }
        command
            .stdin(if debug { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            format!("failed to launch vetrace-player: {error}")
        })?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (sender, receiver) = mpsc::channel();
        if let Some(stdout) = stdout {
            spawn_output_reader(stdout, PlayerOutputStream::Stdout, sender.clone());
        }
        if let Some(stderr) = stderr {
            spawn_output_reader(stderr, PlayerOutputStream::Stderr, sender);
        }

        self.last_exit = None;
        self.cleanup_file = cleanup_file;
        self.stdin = stdin;
        self.events = Some(receiver);
        self.child = Some(child);
        Ok(())
    }

    pub fn drain_events(&mut self) -> Vec<PlayerProcessEvent> {
        let Some(receiver) = self.events.as_ref() else { return Vec::new(); };
        receiver.try_iter().collect()
    }

    pub fn send_debugger_command(&mut self, command: &LuaDebuggerCommand) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or_else(|| "the player debugger is not connected".to_owned())?;
        let payload = serde_json::to_string(command)
            .map_err(|error| format!("failed to encode debugger command: {error}"))?;
        writeln!(stdin, "{DEBUG_COMMAND_PREFIX}{payload}")
            .and_then(|_| stdin.flush())
            .map_err(|error| format!("failed to send debugger command: {error}"))
    }

    pub fn take_exit_status(&mut self) -> Option<ExitStatus> {
        self.last_exit.take()
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.stdin = None;
        let Some(mut child) = self.child.take() else { return Ok(()); };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.last_exit = Some(status);
                self.cleanup_play_file();
                return Ok(());
            }
            Ok(None) => {}
            Err(error) => return Err(format!("failed to query vetrace-player: {error}")),
        }
        child
            .kill()
            .map_err(|error| format!("failed to stop vetrace-player: {error}"))?;
        self.last_exit = child.wait().ok();
        self.cleanup_play_file();
        Ok(())
    }

    fn cleanup_play_file(&mut self) {
        if let Some(path) = self.cleanup_file.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Default for PlayerProcess {
    fn default() -> Self { Self::new() }
}

impl Drop for PlayerProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn spawn_output_reader<R>(reader: R, stream: PlayerOutputStream, sender: Sender<PlayerProcessEvent>)
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let text = match line {
                Ok(line) => line,
                Err(error) => {
                    let _ = sender.send(PlayerProcessEvent::Output(PlayerOutputLine {
                        stream: PlayerOutputStream::Stderr,
                        text: format!("failed to read player output: {error}"),
                    }));
                    break;
                }
            };
            if stream == PlayerOutputStream::Stdout {
                if let Some(payload) = text.strip_prefix(DEBUG_EVENT_PREFIX) {
                    match serde_json::from_str::<LuaDebuggerEvent>(payload) {
                        Ok(event) => {
                            if sender.send(PlayerProcessEvent::Debugger(event)).is_err() { break; }
                            continue;
                        }
                        Err(error) => {
                            let _ = sender.send(PlayerProcessEvent::Output(PlayerOutputLine {
                                stream: PlayerOutputStream::Stderr,
                                text: format!("invalid debugger event from player: {error}"),
                            }));
                            continue;
                        }
                    }
                }
            }
            if sender.send(PlayerProcessEvent::Output(PlayerOutputLine { stream, text })).is_err() {
                break;
            }
        }
    });
}

fn resolve_player_executable() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VETRACE_PLAYER") {
        let path = PathBuf::from(path);
        if player_supports_required_capabilities(&path) { return Some(path); }
    }
    let current = std::env::current_exe().ok()?;
    let directory = current.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["vetrace-player.exe", "vetrace_player.exe"]
    } else {
        &["vetrace-player", "vetrace_player"]
    };
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|path| player_supports_required_capabilities(path))
}

fn player_supports_required_capabilities(path: &Path) -> bool {
    if !path.is_file() { return false; }
    let Ok(output) = Command::new(path).arg("--runtime-info").output() else { return false; };
    output.status.success() && runtime_info_has_required_capabilities(&output.stdout)
}

fn runtime_info_has_required_capabilities(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { return false; };
    value
        .get("capabilities")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|capabilities| {
            REQUIRED_PLAYER_CAPABILITIES.iter().all(|required| {
                capabilities.iter().any(|capability| capability.as_str() == Some(*required))
            })
        })
}

fn resolve_development_workspace() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    for ancestor in current.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.is_file() {
            let source = std::fs::read_to_string(&manifest).ok()?;
            if source.contains("vetrace_player") && source.contains("[workspace]") {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    let current_dir = std::env::current_dir().ok()?;
    for ancestor in current_dir.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.is_file() {
            let source = std::fs::read_to_string(&manifest).ok()?;
            if source.contains("vetrace_player") && source.contains("[workspace]") {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn output_line_preserves_stream() {
        let line = PlayerOutputLine {
            stream: PlayerOutputStream::Stderr,
            text: "problem".to_string(),
        };
        assert_eq!(line.stream, PlayerOutputStream::Stderr);
        assert_eq!(line.text, "problem");
    }

    #[test]
    fn output_reader_forwards_complete_lines() {
        let (sender, receiver) = mpsc::channel();
        spawn_output_reader(
            Cursor::new(b"first\nsecond\n".to_vec()),
            PlayerOutputStream::Stdout,
            sender,
        );

        let first = receiver.recv().unwrap();
        let second = receiver.recv().unwrap();
        let PlayerProcessEvent::Output(first) = first else { panic!("expected output"); };
        let PlayerProcessEvent::Output(second) = second else { panic!("expected output"); };
        assert_eq!(first.text, "first");
        assert_eq!(second.text, "second");
        assert_eq!(first.stream, PlayerOutputStream::Stdout);
        assert_eq!(second.stream, PlayerOutputStream::Stdout);
    }

    #[test]
    fn runtime_info_requires_all_studio_capabilities() {
        assert!(runtime_info_has_required_capabilities(
            br#"{"protocol":1,"capabilities":["screen_ui_text_v1","lua_project_modules_v1"]}"#,
        ));
        assert!(!runtime_info_has_required_capabilities(
            br#"{"protocol":1,"capabilities":["lua_ui_buttons_v1"]}"#,
        ));
        assert!(!runtime_info_has_required_capabilities(
            br#"{"protocol":1,"capabilities":["lua_project_modules_v1"]}"#,
        ));
        assert!(!runtime_info_has_required_capabilities(b"not json"));
    }
}
