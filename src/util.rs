static ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

static COD1_1_SUM: &str = "753fbcabd0fdda7f7dad3dbb29c3c008";
static COD1_5_SUM: &str = "4bdf293d8e6fb32208d1b0942a1ba6bc";
static COD1_5_1_SUM: &str = "928dd08dc169bd85fdd12d2db28def70";

static STATUS_OK: &str      = "[   \x1b[1;92m OK \x1b[0m   ]";
static STATUS_FAILED: &str  = "[ \x1b[1;91m FAILED \x1b[0m ]";

use std::sync::OnceLock;
//use std::sync::atomic::{AtomicBool, Ordering};

static RESOLUTION: OnceLock<(u32, u32)> = OnceLock::new();
static REFRESH_RATE: OnceLock<f32> = OnceLock::new();
pub(crate) static DISPLAY_OUTPUT: OnceLock<String> = OnceLock::new();

//pub(crate) static GAME_RUNNING: AtomicBool = AtomicBool::new(false);

use std::{io, env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use std::collections::BTreeMap;
use io::{Read, Write};

use md5::Context;

use crate::GameInfo;

pub(crate) fn get_exes() -> io::Result<Vec<PathBuf>>
{
    let mut executables = Vec::new();
    let exe_dir = my_exe_path()?;

    let cod_path = exe_dir.join("CoDMP.exe");
    let coduo_path = exe_dir.join("CoDUOMP.exe");
    let iw1x_path = exe_dir.join("iw1x.exe");
    let t1x_path = exe_dir.join("t1x.exe");
    let paths = vec![cod_path, coduo_path, iw1x_path, t1x_path];

    for path in paths {
        if path.exists() {
            executables.push(path);
        }
    }

    Ok(executables)
}

pub(crate) fn my_exe_path() -> io::Result<PathBuf>
{
    // Get the current executable path
    let exe_path = env::current_exe().unwrap();
    let resolved_path = fs::read_link(&exe_path).unwrap_or(exe_path); // Resolve symbolic link if it exists
    let exe_dir = resolved_path.parent().unwrap();

    Ok(exe_dir.to_path_buf())
}

pub(crate) fn name_from_path(executable: &Path) -> String
{
    let path = Path::new(executable);
    let file_name = path.file_name().unwrap();
    file_name.to_string_lossy().to_string()
}

pub(crate) fn name_version_info(executable: &Path) -> Result<Vec<(String, String)>, String>
{
    let exe_name = name_from_path(executable);
    let result: Vec<(String, String)> = match exe_name.to_lowercase().as_str() {

        // Call of Duty
        "codmp.exe" => if verify_file(COD1_1_SUM, executable).unwrap() {
            vec![(String::from("Call of Duty"), String::from("1.1"))]
        } else if verify_file(COD1_5_SUM, executable).unwrap() {
            vec![(String::from("Call of Duty"), String::from("1.5"))]
        } else {
            vec![(String::from("CoDMP"), String::from("???"))]
        },

        // Call of Duty: United Offensive
        "coduomp.exe" => if verify_file(COD1_5_1_SUM, executable).unwrap() {
            vec![(String::from("United Offensive"), String::from("1.51"))]
        } else {
            vec![(String::from("CoDUOMP"), String::from("???"))]
        },

        // Client Extensions
        "iw1x.exe" => vec![(String::from("IW1X"), String::from("1.1"))],
        "t1x.exe" => vec![(String::from("T1X"), String::from("1.51"))],
        _ => vec![(exe_name.to_string(), String::from("???"))],
    };

    Ok(result)
}

pub(crate) fn create_desktop_file(uo: &bool, executable_path: &str) -> std::io::Result<()>
{
    let app_name = if *uo { "CoDLinux (uo)" } else { "CoDLinux" };
    let desktop_file_content = format!(
        "[Desktop Entry]
Type=Application
Name={}
GenericName={}
Exec={executable_path}/codlinux %u
Path={workdir}
Icon=codlinux
Terminal=false
Categories=Game;
StartupNotify=false
Keywords=cod;gaming;wine;
MimeType=x-scheme-handler/iw1x;x-scheme-handler/t1x;
",
        app_name,
        app_name,
        executable_path = executable_path,
        workdir = Path::new(executable_path).parent().unwrap().to_string_lossy().to_string()
    );

    let desktop_file_name = format!("{}.desktop", app_name.replace(" ", "_"));
    let desktop_file_path = std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/applications")
        .join(desktop_file_name);

    fs::write(desktop_file_path, desktop_file_content)?;

    Ok(())
}

pub(crate) fn extract_icon() -> std::io::Result<()>
{
    let icon_file = Path::new(&env::var("HOME").unwrap()).join(".local/share/icons/codlinux.png");

    if !icon_file.exists() {
        println!("Creating icon");
        fs::write(icon_file, ICON_PNG)?;
    }
    Ok(())
}

pub(crate) fn reg_uri_scheme(uri: &str) -> std::io::Result<()>
{
    let desktop_file = match uri {
        "iw1x" => "CoDLinux.desktop".to_string(),
        "t1x" => "CoDLinux_(uo).desktop".to_string(),
        _ => "CoDLinux.desktop".to_string(),
    };
    let cmd = format!(
        "xdg-mime default '{desktop_file}' x-scheme-handler/{uri}",
        desktop_file = desktop_file,
        uri = uri
    );
    Ok(exec_command(&cmd)?)
}

pub(crate) fn exec_command(cmd: &str) -> io::Result<()>
{
    println!("exec_command: {}", cmd);
    let output = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to exec command: {}", cmd);
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to exec shell command"));
    }
    Ok(())
}

pub(crate) fn launch_game(game: &GameInfo) -> std::io::Result<()>
{
    let wine_prefix = if game.wineprefix.trim().is_empty() {
        //String::from("$HOME/.wine")

        let wineprefix = load_setting("default_wine_prefix").unwrap_or_else(|_| {
            save_setting("default_wine_prefix", "$HOME/.wine").unwrap();
            "$HOME/.wine".to_string()
        });

        wineprefix
    }
    else {
        game.wineprefix.to_string()
    };

    let envars = if game.envars.is_empty() {
        String::from("MESA_EXTENSION_MAX_YEAR=2008 force_s3tc_enable=true __GL_ExtensionStringVersion=17700")
    }
    else {
        game.envars.to_string()
    };

    let args = if game.args.is_empty() {
        String::from("+set r_ignorehwgamma 1")
    }
    else {
        game.args.to_string()
    };

    let cmd = format!(
        "WINEPREFIX=\"{wine_prefix}\" {envars} wine {} {args}", game.path.to_string_lossy().to_string());
    //GAME_RUNNING.store(true, Ordering::Relaxed);
    exec_command(&cmd).unwrap();
    //GAME_RUNNING.store(false, Ordering::Relaxed);
    restore_display_mode().unwrap();

    Ok(())
}

pub(crate) fn verify_file(expected: &str, fpath: &Path) -> io::Result<bool>
{
    print!("[    --    ] Verifying file: {} ", &fpath.to_str().unwrap());
    io::stdout().flush()?;
    if !fpath.is_file() {
        println!("\r{} Verifying file: {}  ", STATUS_FAILED, &fpath.to_str().unwrap());
        return Ok(false);
    }
    let mut file = File::open(fpath)?;
    let mut context = Context::new();
    let mut buffer = [0; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        context.consume(&buffer[..bytes_read]);
    }

    let hash = format!("{:x}", context.finalize());
    //println!("{}", hash.as_str());
    if hash != expected {
        println!("\r{} Verifying file: {}  ", STATUS_FAILED, &fpath.to_str().unwrap());
        return Ok(false);
    }
    println!("\r{} Verifying file: {}  ", STATUS_OK, &fpath.to_str().unwrap());
    Ok(true)
}

pub(crate) fn get_display_mode() -> Option<(u32, u32, f32)>
{
    let output = std::process::Command::new("xrandr")
        .arg("--current")
        .output()
        .expect("Failed to execute xrandr");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(" connected") {
            let tok:Vec<&str> = line.split_whitespace().collect();
            let output_name = tok[0];
            DISPLAY_OUTPUT.set(output_name.to_string()).ok();
            println!("Output: {}", output_name);
        }
        if line.contains("*") {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let resolution = tokens[0];
            // Find the token that contains "*"
            let mode_token = tokens.iter().find(|&t| t.contains("*")).expect("No token contains '*'");
            // Extract the numeric part (keep digits and decimal points)
            let refresh_rate_str: String = mode_token.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
            let refresh_rate: f32 = refresh_rate_str.parse().expect("Failed to parse refresh rate");
            // Parse resolution
            let parts: Vec<&str> = resolution.split('x').collect();
            let width = parts[0].parse::<u32>().expect("Failed to parse width");
            let height = parts[1].parse::<u32>().expect("Failed to parse height");

            RESOLUTION.set((width, height)).ok();
            REFRESH_RATE.set(refresh_rate).ok();
            return Some((width, height, refresh_rate));
        }
    }
    None
}

pub(crate) fn restore_display_mode() -> std::io::Result<()>
{
    let Some((width, height)) = RESOLUTION.get() else {
        eprintln!("Error: Resolution not set.");
        return Ok(());
    };
    let Some(rate) = REFRESH_RATE.get() else {
        eprintln!("Error: Refresh rate not set.");
        return Ok(());
    };

    let output = std::process::Command::new("xrandr")
        .arg("--output")
        .arg(DISPLAY_OUTPUT.get().unwrap())
        .arg("--mode")
        .arg(format!("{}x{}", width, height))
        .arg("--rate")
        .arg(format!("{}", rate))
        .output()?;

    if !output.status.success() {
        eprintln!("Error restoring resolution: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn read_config(config_file: &PathBuf) -> std::io::Result<BTreeMap<String, String>>
{
    //let config_file = my_exe_path().unwrap().join("codlinux/codlinux.cfg");

    // If the file doesn't exist, return an empty HashMap
    if !config_file.exists() {
        return Ok(BTreeMap::new());
    }

    // Read the file contents
    let contents = fs::read_to_string(config_file)?;
    let mut config = BTreeMap::new();

    // Parse each line as a key-value pair
    for line in contents.lines() {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            config.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    Ok(config)
}

// Helper function to write the HashMap back to the configuration file
fn write_config(config_file: &PathBuf, config: &BTreeMap<String, String>) -> std::io::Result<()> {
    //let config_file = my_exe_path().unwrap().join("codlinux.cfg");
    let mut file = fs::File::create(config_file)?;

    // Write each key-value pair as "key=value"
    for (key, value) in config {
        writeln!(file, "{}={}", key, value)?;
    }
    Ok(())
}

pub(crate) fn save_setting(seting: &str, value: &str) -> std::io::Result<()>
{
    let path = my_exe_path().unwrap().join("codlinux_conf/codlinux.cfg");
    let mut config = read_config(&path)?;
    config.insert(seting.to_string(), value.to_string());
    write_config(&path, &config)
}

pub(crate) fn load_setting(setting: &str) -> std::io::Result<String>
{
    let config = read_config(&my_exe_path().unwrap().join("codlinux_conf/codlinux.cfg"))?;
    Ok(config.get(setting).cloned().unwrap_or_default())
}

pub(crate) fn save_game_config(game: &str, config: &BTreeMap<String, String>) -> std::io::Result<()>
{
    let path = my_exe_path().unwrap().join(format!("codlinux_conf/{game}.cfg"));
    //let mut config = read_config(&path)?;
    //config.insert(seting.to_string(), value.to_string());
    write_config(&path, &config)
}

pub(crate) fn get_game_config(game: &str) -> std::io::Result<BTreeMap<String, String>>
{
    let config = read_config(&my_exe_path().unwrap().join(format!("codlinux_conf/{game}.cfg")))?;
    Ok(config)
}

pub(crate) fn notify(message: &str, expire_time: u32, transient: bool) -> std::io::Result<()>
{
    let cmd = if transient {
        format!(r#"notify-send --app-name=CoDLinux --icon=codlinux --transient --expire-time={} CoDLinux "{}""#, expire_time, message)
    }
    else {
        format!(r#"notify-send --app-name=CoDLinux --icon=codlinux --expire-time={} CoDLinux "{}""#, expire_time, message)
    };

    exec_command(&cmd).unwrap();
    Ok(())
}
