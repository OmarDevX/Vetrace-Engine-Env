use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use zip::write::SimpleFileOptions;

use crate::{BuildError, BuildResult, PLAYER_TEMPLATE_METADATA_SUFFIX};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DistributionArtifact {
    PortableArchive(PathBuf),
    WindowsInstaller(PathBuf),
    LinuxAppImage(PathBuf),
    MacApplication(PathBuf),
}

pub fn package_portable_zip(
    build_directory: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> BuildResult<DistributionArtifact> {
    let build_directory = build_directory.as_ref();
    let destination = destination.as_ref();
    let parent = destination.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| BuildError::io("create package output directory", parent, error))?;
    let temporary = destination.with_extension("zip.tmp");
    let file = fs::File::create(&temporary)
        .map_err(|error| BuildError::io("create portable package", &temporary, error))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    append_directory(&mut writer, build_directory, build_directory, options)?;
    writer.finish()?;
    replace_file(&temporary, destination)?;
    Ok(DistributionArtifact::PortableArchive(destination.to_path_buf()))
}



/// Builds a Windows installer with a caller-supplied NSIS `makensis` binary.
///
/// The exported game directory is embedded recursively. The installer creates
/// Start Menu and optional desktop shortcuts and includes an uninstaller. This
/// function never invokes Cargo and can be used from a Windows release host or
/// a configured cross-packaging environment containing NSIS.
pub fn package_windows_nsis(
    build_directory: impl AsRef<Path>,
    application_name: &str,
    destination: impl AsRef<Path>,
    makensis: impl AsRef<Path>,
) -> BuildResult<DistributionArtifact> {
    let build_directory = build_directory.as_ref();
    let destination = destination.as_ref();
    if !build_directory.is_dir() {
        return Err(BuildError::Validation(format!(
            "Windows installer source '{}' is not a directory",
            build_directory.display()
        )));
    }
    let executable = first_windows_executable(build_directory).ok_or_else(|| {
        BuildError::Validation("export directory contains no Windows .exe player".into())
    })?;
    let executable_name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BuildError::Validation("exported executable name is not valid UTF-8".into()))?;
    let parent = destination.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| BuildError::io("create Windows installer output directory", parent, error))?;

    let script_path = parent.join(format!(
        ".{}.installer.nsi",
        sanitize_file_stem(application_name)
    ));
    let install_name = nsis_escape(application_name);
    let source = nsis_escape(&absolute_path(build_directory)?);
    let output = nsis_escape(&absolute_path(destination)?);
    let executable_name = nsis_escape(executable_name);
    let script = format!(r#"Unicode true
Name "{install_name}"
OutFile "{output}"
InstallDir "$LOCALAPPDATA\Programs\{install_name}"
RequestExecutionLevel user
SetCompressor /SOLID lzma

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install" SecInstall
  SetOutPath "$INSTDIR"
  File /r "{source}\*.*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\{install_name}"
  CreateShortcut "$SMPROGRAMS\{install_name}\{install_name}.lnk" "$INSTDIR\{executable_name}"
  CreateShortcut "$SMPROGRAMS\{install_name}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\{install_name}.lnk" "$INSTDIR\{executable_name}"
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\{install_name}.lnk"
  RMDir /r "$SMPROGRAMS\{install_name}"
  RMDir /r "$INSTDIR"
SectionEnd
"#);
    fs::write(&script_path, script)
        .map_err(|error| BuildError::io("write NSIS installer script", &script_path, error))?;
    let status = Command::new(makensis.as_ref())
        .arg(&script_path)
        .status()
        .map_err(|error| BuildError::io("launch makensis", makensis.as_ref(), error))?;
    let _ = fs::remove_file(&script_path);
    if !status.success() {
        return Err(BuildError::Validation(format!("makensis exited with {status}")));
    }
    if !destination.is_file() {
        return Err(BuildError::Validation(format!(
            "makensis completed but did not create '{}'",
            destination.display()
        )));
    }
    Ok(DistributionArtifact::WindowsInstaller(destination.to_path_buf()))
}

pub fn package_linux_appimage(
    build_directory: impl AsRef<Path>,
    application_name: &str,
    destination: impl AsRef<Path>,
    appimagetool: impl AsRef<Path>,
) -> BuildResult<DistributionArtifact> {
    let build_directory = build_directory.as_ref();
    let destination = destination.as_ref();
    let app_dir = destination.with_extension("AppDir");
    if app_dir.exists() {
        fs::remove_dir_all(&app_dir)
            .map_err(|error| BuildError::io("clear AppDir", &app_dir, error))?;
    }
    let usr_bin = app_dir.join("usr/bin");
    fs::create_dir_all(&usr_bin)
        .map_err(|error| BuildError::io("create AppDir", &usr_bin, error))?;
    copy_tree(build_directory, &usr_bin)?;
    let executable = first_executable(&usr_bin).ok_or_else(|| {
        BuildError::Validation("export directory contains no executable player".into())
    })?;
    let executable_name = executable.file_name().unwrap_or_default().to_string_lossy();
    let app_run = format!("#!/bin/sh\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$HERE/usr/bin/{executable_name}\" \"$@\"\n");
    let app_run_path = app_dir.join("AppRun");
    fs::write(&app_run_path, app_run)
        .map_err(|error| BuildError::io("write AppRun", &app_run_path, error))?;
    make_executable(&app_run_path)?;
    let desktop = format!("[Desktop Entry]\nType=Application\nName={application_name}\nExec={executable_name}\nIcon=vetrace-game\nCategories=Game;\nTerminal=false\n");
    fs::write(app_dir.join("vetrace-game.desktop"), desktop)
        .map_err(|error| BuildError::io("write AppImage desktop file", &app_dir, error))?;
    let status = Command::new(appimagetool.as_ref())
        .arg(&app_dir)
        .arg(destination)
        .status()
        .map_err(|error| BuildError::io("launch appimagetool", appimagetool.as_ref(), error))?;
    if !status.success() {
        return Err(BuildError::Validation(format!("appimagetool exited with {status}")));
    }
    let _ = fs::remove_dir_all(&app_dir);
    Ok(DistributionArtifact::LinuxAppImage(destination.to_path_buf()))
}

pub fn package_macos_app(
    build_directory: impl AsRef<Path>,
    application_name: &str,
    destination: impl AsRef<Path>,
) -> BuildResult<DistributionArtifact> {
    let destination = destination.as_ref();
    let contents = destination.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    if destination.exists() {
        fs::remove_dir_all(destination)
            .map_err(|error| BuildError::io("clear macOS application bundle", destination, error))?;
    }
    fs::create_dir_all(&macos)
        .map_err(|error| BuildError::io("create macOS application bundle", &macos, error))?;
    fs::create_dir_all(&resources)
        .map_err(|error| BuildError::io("create macOS application resources", &resources, error))?;
    copy_tree(build_directory.as_ref(), &macos)?;
    let executable = first_executable(&macos).ok_or_else(|| BuildError::Validation("export directory contains no executable player".into()))?;
    let executable_name = executable.file_name().unwrap_or_default().to_string_lossy();
    let application_name = xml_escape(application_name);
    let executable_name = xml_escape(&executable_name);
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>{application_name}</string>
<key>CFBundleDisplayName</key><string>{application_name}</string>
<key>CFBundleExecutable</key><string>{executable_name}</string>
<key>CFBundleIdentifier</key><string>engine.vetrace.exported-game</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>"#);
    fs::write(contents.join("Info.plist"), plist)
        .map_err(|error| BuildError::io("write macOS Info.plist", &contents, error))?;
    Ok(DistributionArtifact::MacApplication(destination.to_path_buf()))
}

fn append_directory(
    writer: &mut zip::ZipWriter<fs::File>,
    root: &Path,
    directory: &Path,
    options: SimpleFileOptions,
) -> BuildResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| BuildError::io("read package directory", directory, error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| BuildError::io("read package entry", directory, error))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type().map_err(|error| BuildError::io("inspect package entry", &path, error))?.is_symlink() { continue; }
        let relative = path.strip_prefix(root).map_err(|_| BuildError::Validation("package path escaped build directory".into()))?;
        let name = relative.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            writer.add_directory(format!("{name}/"), options)?;
            append_directory(writer, root, &path, options)?;
        } else {
            writer.start_file(name, options)?;
            let mut file = fs::File::open(&path)
                .map_err(|error| BuildError::io("open package entry", &path, error))?;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer)
                    .map_err(|error| BuildError::io("read package entry", &path, error))?;
                if read == 0 { break; }
                writer.write_all(&buffer[..read])
                    .map_err(|error| BuildError::io("write portable package", root, error))?;
            }
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> BuildResult<()> {
    fs::create_dir_all(destination)
        .map_err(|error| BuildError::io("create distribution directory", destination, error))?;
    for entry in fs::read_dir(source)
        .map_err(|error| BuildError::io("read distribution source", source, error))?
    {
        let entry = entry.map_err(|error| BuildError::io("read distribution entry", source, error))?;
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type()
            .map_err(|error| BuildError::io("inspect distribution entry", entry.path(), error))?;
        if file_type.is_symlink() { continue; }
        if file_type.is_dir() { copy_tree(&entry.path(), &target)?; }
        else { fs::copy(entry.path(), &target).map_err(|error| BuildError::io("copy distribution entry", &target, error))?; }
    }
    Ok(())
}

fn first_windows_executable(directory: &Path) -> Option<PathBuf> {
    fs::read_dir(directory).ok()?.flatten().map(|entry| entry.path()).find(|path| {
        path.is_file()
            && path.extension().and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            && path.file_name().and_then(|name| name.to_str())
                .is_some_and(|name| !name.eq_ignore_ascii_case("uninstall.exe"))
    })
}

fn first_executable(directory: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_player_candidate(path))
        .collect::<Vec<_>>();
    candidates.sort();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(path) = candidates.iter().find(|path| {
            fs::metadata(path)
                .ok()
                .is_some_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
        }) {
            return Some(path.clone());
        }
    }
    candidates.into_iter().find(|path| path.extension().is_none())
}

fn is_player_candidate(path: &Path) -> bool {
    if !path.is_file() { return false; }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else { return false; };
    !name.ends_with(PLAYER_TEMPLATE_METADATA_SUFFIX)
        && name != "game.vpak"
        && name != "build-report.json"
        && name != "manifest.json"
        && !matches!(path.extension().and_then(|extension| extension.to_str()), Some("json" | "toml" | "txt" | "md" | "html"))
}

fn replace_file(source: &Path, destination: &Path) -> BuildResult<()> {
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| BuildError::io("replace distribution artifact", destination, error))?;
    }
    fs::rename(source, destination)
        .map_err(|error| BuildError::io("replace distribution artifact", destination, error))
}

fn make_executable(path: &Path) -> BuildResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|error| BuildError::io("read executable permissions", path, error))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)
            .map_err(|error| BuildError::io("set executable permissions", path, error))?;
    }
    Ok(())
}


fn absolute_path(path: &Path) -> BuildResult<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| BuildError::io("resolve current directory", Path::new("."), error))?
            .join(path)
    };
    Ok(absolute.to_string_lossy().replace('/', "\\"))
}

fn nsis_escape(value: &str) -> String {
    value
        .replace('$', "$$")
        .replace('"', "$\\\"")
        .replace('\r', " ")
        .replace('\n', " ")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sanitize_file_stem(value: &str) -> String {
    let value = value
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') { character } else { '-' })
        .collect::<String>();
    let value = value.trim_matches('-');
    if value.is_empty() { "vetrace-game".to_owned() } else { value.to_owned() }
}
