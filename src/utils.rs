const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

use std::sync::OnceLock;

static RESOLUTION: OnceLock<(u32, u32)> = OnceLock::new();
static REFRESH_RATE: OnceLock<f32> = OnceLock::new();
static DISPLAY_OUTPUT: OnceLock<String> = OnceLock::new();

use std::fs;
use std::path::Path;
use std::io::Write;
use std::collections::HashMap;

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
        eprintln!("Error executing command: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

pub(crate) fn launch_game(wine_prefix: &str, executable: &str, args: &str) -> std::io::Result<()>
{
    let cmd = format!(
        "WINEPREFIX={} MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}",
        wine_prefix, executable, args
    );
    exec_command(&cmd).unwrap();
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

// Save the remembered game path
pub(crate) fn remember_game(game: &str) -> std::io::Result<()> {
    let mut config = read_config()?;
    config.insert("remembered_game".to_string(), game.to_string());
    write_config(&config)
}

// Retrieve the remembered game path
pub(crate) fn recall_game() -> std::io::Result<String> {
    let config = read_config()?;
    Ok(config.get("remembered_game").cloned().unwrap_or_default())
}

// Save the Wine prefix
pub(crate) fn save_wine_prefix(prefix: &str) -> std::io::Result<()> {
    let mut config = read_config()?;
    config.insert("wine_prefix".to_string(), prefix.to_string());
    write_config(&config)
}

// Retrieve the Wine prefix
pub(crate) fn recall_wine_prefix() -> std::io::Result<String> {
    let config = read_config()?;
    Ok(config.get("wine_prefix").cloned().unwrap_or_default())
}