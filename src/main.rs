use utils::{create_desktop_file, my_exe_path, get_fancy_name, reg_uri_scheme, exec_command};

mod utils;

use eframe::egui;

fn main()
{
    println!("CoDLinux: Looking for game executables...");
    let mut uo = false;
    let mut cod1 = false;
    let mut iw1x = false;
    let mut t1x = false;
    let mut launched = false;
    let executables = utils::get_executables();
    if executables.is_empty() {
        println!("CoDLinux: No game executables found.");
        return;
    }
    println!("CoDLinux: Found game executables:");
    for executable in &executables {
        println!("  -{}", executable);
    }

    for executable in &executables {
        let exe_name = utils::get_exe_name(executable);
        if exe_name.to_lowercase() == "codmp.exe" {
            cod1 = true;
        }
        else if exe_name.to_lowercase() == "coduomp.exe" {
            uo = true;
        }
        else if exe_name.to_lowercase() == "iw1x.exe" {
            iw1x = true;
        }
        else if exe_name.to_lowercase() == "t1x.exe" {
            t1x = true;
        }
    }

    utils::extract_icon().unwrap();

    if uo {
        create_desktop_file(&uo, &my_exe_path().to_string_lossy().to_string()).unwrap();
        if t1x {
            reg_uri_scheme("t1x").unwrap();
        }
    }
    if cod1 && !uo {
        create_desktop_file(&uo, &my_exe_path().to_string_lossy().to_string()).unwrap();
        if iw1x {
            reg_uri_scheme("iw1x").unwrap();
        }
    }

    let mut args:Vec<String> = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(first_arg) = args.get(0) {
        if first_arg.starts_with("iw1x://") || first_arg.starts_with("t1x://") {
            // Parse the IP and port from the argument
            let _match = if first_arg.starts_with("iw1x://") { "iw1x://" } else { "t1x://" };
            let stripped = first_arg.trim_start_matches(_match);
            let parts: Vec<&str> = stripped.split(':').collect();
            let ip = parts.get(0).unwrap_or(&"127.0.0.1"); // Default to 127.0.0.1 if no IP is provided
            let port = parts.get(1).unwrap_or(&"28960");   // Default to 28960 if no port is provided

            // Replace the argument with "+connect ip:port"
            args[0] = format!("+connect {}:{}", ip, port);

            let args_str = args.join(" ");
            if iw1x {
                for exe in &executables {
                    if exe.to_lowercase().contains("iw1x.exe") {
                        exec_command(&format!("MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}", exe, args_str)).unwrap();
                        launched = true;
                    }
                }
            }
            if t1x {
                for exe in &executables {
                    if exe.to_lowercase().contains("t1x.exe") {
                        exec_command(&format!("MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}", exe, args_str)).unwrap();
                        launched = true;
                    }
                }
            }
        }
    }

    if !launched {
        println!("CoDLinux: Launching GUI...");
        let args_str = args.join(" ");
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 120.0 + ((120.0 * executables.len() as f32) - 120.0)]),
            ..Default::default()
        };
        eframe::run_native(
            "CoDLinux",
            options,
            Box::new(|_cc| {
                Ok(Box::new(CoDLinuxApp::new(executables, args_str, uo)))
            }),
        ).unwrap();
    }
}

pub struct CoDLinuxApp
{
    executables: Vec<String>,
    args: String,
    uo: bool,
}

impl CoDLinuxApp
{
    fn new(executables: Vec<String>, args: String, uo: bool) -> Self {
        CoDLinuxApp {
            executables,
            args,
            uo,
        }
    }
}

impl eframe::App for CoDLinuxApp
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.visuals.widgets.hovered.bg_stroke.color = egui::Color32::from_rgb(180, 127, 240);
        style.visuals.widgets.active.bg_stroke.color = egui::Color32::from_rgb(187, 220, 61);
        style.visuals.widgets.hovered.bg_stroke.width = 1.5;
        style.visuals.widgets.active.bg_stroke.width = 2.0;
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Choose a Game");
                for executable in &self.executables {
                    let game_name = get_fancy_name(executable, &self.uo);

                    let text = egui::RichText::new(game_name).size(24.0).strong();
                    let button = egui::Button::new(text).min_size(egui::vec2(300.0, 100.0));

                    if ui.add(button).clicked() {

                        // Run the command in a separate thread
                        let executable_clone = executable.clone();
                        let args_clone = self.args.clone();
                        std::thread::spawn(move || {
                            let command = format!(
                                "MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}",
                                executable_clone, args_clone
                            );
                            if let Err(e) = exec_command(&command) {
                                eprintln!("Failed to execute command: {}", e);
                            }
                        });

                        // close the GUI
                        std::process::exit(0);
                    }
                }
            });
        });
    }
}
