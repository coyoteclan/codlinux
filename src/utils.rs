const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static RESOLUTION: OnceLock<(u32, u32)> = OnceLock::new();
static REFRESH_RATE: OnceLock<f32> = OnceLock::new();
static DISPLAY_OUTPUT: OnceLock<String> = OnceLock::new();
pub(crate) static GAME_RUNNING: AtomicBool = AtomicBool::new(false);
pub(crate) static DL_STARTED: AtomicBool = AtomicBool::new(false);
pub(crate) static DL_DONE: AtomicBool = AtomicBool::new(false);
pub(crate) static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);

use std::fs;
use std::path::Path;
use std::io::Write;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use sysinfo::System;

pub(crate) fn get_executables() -> Vec<String>
{
    let mut executables = Vec::new();
    let exe_dir = my_exe_path();

    let cod_path = exe_dir.join("CoDMP.exe");
    let coduo_path = exe_dir.join("CoDUOMP.exe");
    let iw1x_path = exe_dir.join("iw1x.exe");
    let t1x_path = exe_dir.join("t1x.exe");
    let paths = vec![cod_path, coduo_path, iw1x_path, t1x_path];

    for path in paths {
        if path.exists() {
            executables.push(path.to_string_lossy().to_string());
        }
    }

    executables
}

pub(crate) fn my_exe_path() -> std::path::PathBuf
{
    // Get the current executable path
    let exe_path = std::env::current_exe().unwrap();
    let resolved_path = fs::read_link(&exe_path).unwrap_or(exe_path); // Resolve symbolic link if it exists
    let exe_dir = resolved_path.parent().unwrap();

    exe_dir.to_path_buf()
}

pub(crate) fn get_exe_name(executable: &str) -> String
{
    let path = std::path::Path::new(executable);
    let file_name = path.file_name().unwrap();
    file_name.to_string_lossy().to_string()
}

pub(crate) fn get_fancy_name(executable: &str, uo: &bool) -> String
{
    let exe_name = get_exe_name(executable);
    let fancy_name = match exe_name.to_lowercase().as_str() {
        "codmp.exe" => if *uo { "Call of Duty (v1.5)" } else { "Call of Duty (v1.1)" },
        "coduomp.exe" => "United Offensive (v1.51)",
        "iw1x.exe" => "iw1x (CoD v1.1)",
        "t1x.exe" => "t1x (UO v1.51)",
        _ => &exe_name,
    };
    fancy_name.to_string()
}

pub(crate) fn create_desktop_file(uo: &bool, executable_path: &str) -> std::io::Result<()>
{
    let app_name = if *uo { "CoDLinux (uo)" } else { "CoDLinux" };
    let desktop_file_content = format!(
"[Desktop Entry]
Type=Application
Name={}
Exec={executable_path}/codlinux %u
Path={workdir}
Icon=codlinux
Terminal=false
Categories=Game;
StartupNotify=false
Keywords=cod;gaming;wine;
",
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
    let home_dir = std::env::var("HOME").unwrap();
    let icon_file = Path::new(&home_dir).join(".local/share/icons/codlinux.png");

    if !icon_file.exists() {
        println!("Creating icon");
        fs::write(icon_file, ICON_PNG)?;
    }
    Ok(())
}

pub(crate) fn exec_command(cmd: &str) -> std::io::Result<()>
{
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .current_dir(my_exe_path())
        .output()?;

    if !output.status.success() {
        eprintln!("Error executing command: stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

pub(crate) fn launch_game(wine_prefix: &str, executable: &str, args: &str) -> std::io::Result<()>
{
    let cmd = format!(
        "WINEPREFIX={} MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}",
        wine_prefix, executable, args
    );
    GAME_RUNNING.store(true, Ordering::Relaxed);
    exec_command(&cmd).unwrap();
    GAME_RUNNING.store(false, Ordering::Relaxed);
    restore_display_mode().unwrap();

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
    exec_command(&cmd)
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
            // Store and return
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

// Helper function to read the configuration file into a HashMap
fn read_config() -> std::io::Result<HashMap<String, String>> {
    let config_path = my_exe_path();
    let config_file = config_path.join("codlinux.cfg");
    
    // If the file doesn't exist, return an empty HashMap
    if !config_file.exists() {
        return Ok(HashMap::new());
    }
    
    // Read the file contents
    let contents = fs::read_to_string(config_file)?;
    let mut config = HashMap::new();
    
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
fn write_config(config: &HashMap<String, String>) -> std::io::Result<()> {
    let config_path = my_exe_path();
    let config_file = config_path.join("codlinux.cfg");
    let mut file = fs::File::create(config_file)?;
    
    // Write each key-value pair as "key=value"
    for (key, value) in config {
        writeln!(file, "{}={}", key, value)?;
    }
    Ok(())
}

pub(crate) fn save_setting(seting: &str, value: &str) -> std::io::Result<()>
{
    let mut config = read_config()?;
    config.insert(seting.to_string(), value.to_string());
    write_config(&config)
}

pub(crate) fn load_setting(setting: &str) -> std::io::Result<String>
{
    let config = read_config()?;
    Ok(config.get(setting).cloned().unwrap_or_default())
}

fn latest_release_date() -> Result<DateTime<Utc>, Box<dyn std::error::Error>>
{
    let cmd = "curl -s https://api.github.com/repos/coyoteclan/codlinux/releases/latest | grep 'published_at'";
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let str_output = stdout.to_string();
    println!("tok_err line {}", str_output);
    let tok:Vec<&str> = str_output.split('"').collect();

    let release_date = DateTime::parse_from_rfc3339(tok[3])?.with_timezone(&Utc);
    println!("release_date: {}", release_date.to_string());

    Ok(release_date)
}

fn get_download_url() -> String
{
    let cmd = "curl -s https://api.github.com/repos/coyoteclan/codlinux/releases/latest | grep 'browser_download_url'";
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let str_output = stdout.to_string();
    let tok:Vec<&str> = str_output.split('"').collect();
    println!("url: {}", &tok[3].to_string());

    tok[3].to_string()
}

pub(crate) fn get_download_size() -> String
{
    let cmd = format!("curl -s https://api.github.com/repos/coyoteclan/codlinux/releases/latest | grep 'size'").to_string();
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let tok:Vec<&str> = stdout.split(":").collect();
    let tok_:Vec<&str> = tok[1].split(",").collect();
    let size = tok_[0].to_string().trim().to_string();
    let mb_size = format!("{:.2} MB", size.parse::<f32>().unwrap() / (1024.0 * 1024.0) as f32);

    mb_size
}

fn get_compile_time() -> Result<DateTime<Utc>, Box<dyn std::error::Error>>
{
    let compile_time = DateTime::parse_from_rfc3339(compile_time::datetime_str!())?.with_timezone(&Utc);
    Ok(compile_time)
}

pub(crate) fn check_update() -> Result<bool, Box<dyn std::error::Error>>
{
    let release_date = latest_release_date()?;
    let compile_time = get_compile_time()?;
    let difference = release_date - compile_time;
    //println!("difference: {}", difference.to_string());
    Ok(difference > chrono::Duration::hours(1))
}

pub(crate) fn dl_update() -> std::io::Result<()>
{
    if DL_STARTED.load(Ordering::Relaxed) || DL_DONE.load(Ordering::Relaxed) {
        return Ok(());
    }
    DL_STARTED.store(true, Ordering::Relaxed);
    let url = get_download_url().to_string();
    let cmd = format!("wget -O codlinux_new {}", &url);

    // Execute the download command
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .current_dir(my_exe_path())
        .output();

    // Check if the command executed successfully
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("Download completed successfully.");
                // Replace the old binary with the new one
                let replace_cmd = "rm codlinux && mv codlinux_new codlinux && chmod +x codlinux";
                let replace_output = std::process::Command::new("bash")
                    .arg("-c")
                    .arg(replace_cmd)
                    .current_dir(my_exe_path())
                    .output();

                if replace_output.is_err() || !replace_output.unwrap().status.success() {
                    eprintln!("Failed to replace the old binary with the new one.");
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to replace the binary.",
                    ));
                }

                DL_DONE.store(true, Ordering::Relaxed);
            } else {
                eprintln!(
                    "Download failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Download failed.",
                ));
            }
        }
        Err(e) => {
            eprintln!("Failed to execute download command: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

pub(crate) fn notify(message: &str, expire_time: u32) -> std::io::Result<()>
{
    let cmd = format!(r#"notify-send --app-name=CoDLinux --icon=codlinux --expire-time {} "{}""#, expire_time, message);
    exec_command(&cmd).unwrap();
    Ok(())
}

pub(crate) fn moss_running() -> std::io::Result<bool>
{
    let s = System::new_all();
    let p = s.processes_by_name("oss".as_ref());
    let i: u32 = p.count() as u32;
    
    if i == 0 {
        return Ok(false);
    }

    Ok(true)
}

pub(crate) fn moss_capturing() -> std::io::Result<bool>
{
    let s = System::new_all();
    let p = s.processes_by_name("oss".as_ref());
    let i: u32 = p.count() as u32;
    
    if i != 5 {
        return Ok(false);
    }

    Ok(true)
}