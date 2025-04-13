const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

use std::fs;
use std::path::Path;

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
    //println!("{}", cmd);

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
