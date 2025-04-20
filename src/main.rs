use std::sync::{Arc, Mutex};
use std::thread;
use eframe::egui;
use utils::{create_desktop_file, my_exe_path, get_fancy_name, reg_uri_scheme, exec_command, launch_game};

mod utils;

fn main()
{
    let resolution = utils::get_display_mode();
    if let Some((width, height, rate)) = resolution {
        println!("CoDLinux: Display resolution: {}x{} {} Hz", width, height, rate);
    } else {
        println!("CoDLinux: Unable to get display resolution.");
    }

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
        } else if exe_name.to_lowercase() == "coduomp.exe" {
            uo = true;
        } else if exe_name.to_lowercase() == "iw1x.exe" {
            iw1x = true;
        } else if exe_name.to_lowercase() == "t1x.exe" {
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

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let backupprefix = format!("{}/.wine", home);

    let mut wineprefix = utils::recall_wine_prefix().unwrap_or_else(|_| {
        backupprefix.clone()
    }
    );
    if &wineprefix == "" {
        wineprefix = backupprefix;
    }

    let mut args: Vec<String> = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(first_arg) = args.get(0) {
        if first_arg.starts_with("iw1x://") || first_arg.starts_with("t1x://") {
            let _match = if first_arg.starts_with("iw1x://") { "iw1x://" } else { "t1x://" };
            let stripped = first_arg.trim_start_matches(_match);
            let parts: Vec<&str> = stripped.split(':').collect();
            let ip = parts.get(0).unwrap_or(&"127.0.0.1");
            let port = parts.get(1).unwrap_or(&"28960");

            args[0] = format!("+connect {}:{}", ip, port);

            let args_str = args.join(" ");
            if iw1x {
                for exe in &executables {
                    if exe.to_lowercase().contains("iw1x.exe") {
                        /*exec_command(&format!(
                            "WINEPREFIX={} MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}",
                            wineprefix, exe, args_str
                        )).unwrap();
                        utils::restore_display_mode().unwrap();*/
                        launch_game(&wineprefix, exe, &args_str).unwrap();
                        launched = true;
                    }
                }
            }
            if t1x {
                for exe in &executables {
                    if exe.to_lowercase().contains("t1x.exe") {
                        /*exec_command(&format!(
                            "MESA_EXTENSION_MAX_YEAR=2003 force_s3tc_enable=true __GL_ExtensionStringVersion=17700 wine {} {}",
                            exe, args_str
                        )).unwrap();
                        utils::restore_display_mode().unwrap();*/

                        launch_game(&wineprefix, exe, &args_str).unwrap();
                        launched = true;
                    }
                }
            }
        }
    }

    if !launched {
        let args_str = args.join(" ");
        let saved_game = utils::recall_game().unwrap();
        if &saved_game != "" {
            launch_game(&wineprefix, &saved_game, &args_str).unwrap();
            launched = true;
        }
    }
    if !launched {
        println!("CoDLinux: Launching GUI...");
        let args_str = args.join(" ");
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 120.0 + ((120.0 * executables.len() as f32) - 120.0) + 40.0]),
            ..Default::default()
        };

        // Create a shared thread handle
        let game_thread = Arc::new(Mutex::new(None::<thread::JoinHandle<()>>));
        let app = CoDLinuxApp::new(executables.clone(), args_str.clone(), uo, game_thread.clone(), wineprefix.clone());

        eframe::run_native(
            "CoDLinux",
            options,
            Box::new(|_cc| Ok(Box::new(app))),
        ).unwrap();

        // Join the game thread after GUI closes
        if let Some(handle) = game_thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
        println!("CoDLinux: GUI closed.");
    }
}

pub struct CoDLinuxApp
{
    executables: Vec<String>,
    args: String,
    uo: bool,
    game_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    remember: bool,
    wine_prefix: String,
}

impl CoDLinuxApp
{
    fn new(executables: Vec<String>, args: String, uo: bool, game_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>, wine_prefix: String) -> Self {
        println!("CoDLinux: Creating app...");
        println!("CoDLinux: Prefix: {:?}", wine_prefix);
        CoDLinuxApp {
            executables,
            args,
            uo,
            game_thread,
            remember: false,
            wine_prefix,
        }
    }
}

impl eframe::App for CoDLinuxApp
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::default();
        visuals.text_cursor = egui::style::TextCursorStyle {
            stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(187, 220, 61)), // Red cursor
            preview: true,
            blink: true,
            on_duration: 0.5,
            off_duration: 0.5,
        };
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.visuals.widgets.hovered.bg_stroke.color = egui::Color32::from_rgb(180, 127, 240);
        style.visuals.widgets.active.bg_stroke.color = egui::Color32::from_rgb(187, 220, 61);
        style.visuals.widgets.open.bg_stroke.color = egui::Color32::from_rgb(187, 220, 61);
        style.visuals.widgets.hovered.bg_stroke.width = 1.5;
        style.visuals.widgets.active.bg_stroke.width = 2.0;
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.visuals_mut().selection.stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(187, 220, 61));
            ui.vertical_centered(|ui| {
                ui.heading("Choose a Game");
                for executable in &self.executables {
                    let game_name = get_fancy_name(executable, &self.uo);

                    let text = egui::RichText::new(&game_name).size(24.0).strong();
                    let button = egui::Button::new(text).min_size(egui::vec2(300.0, 100.0));

                    if ui.add(button).clicked() {
                        if self.remember {
                            println!("CoDLinux: Remembering choice for {}, path: {}", &game_name, &executable);
                            let _ = utils::remember_game(&executable);
                        }
                        let exe = executable.clone();
                        let args = self.args.clone();

                        let wine_prefix = self.wine_prefix.clone();
                        let game_handle = thread::spawn(move || {
                            //let _ = utils::save_wine_prefix(&wine_prefix);
                            let rrr = utils::save_wine_prefix(&wine_prefix);
                            if rrr.is_err() {
                                println!("CoDLinux: Error saving wine prefix: {}", rrr.unwrap_err());
                            }
                            println!("CoDLinux: prefix: {}", &wine_prefix);
                            let _ = launch_game(&wine_prefix, &exe, &args);
                        });

                        *self.game_thread.lock().unwrap() = Some(game_handle);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }

                ui.checkbox(&mut self.remember, "Remember my choice");
                let text_edit = ui.add(egui::TextEdit::singleline(&mut self.wine_prefix)
                    .hint_text("Wine Prefix")
                    .desired_width(200.0));
                
                if text_edit.changed() {
                    if let Ok(new_prefix) = std::fs::canonicalize(&self.wine_prefix) {
                        self.wine_prefix = new_prefix.to_string_lossy().to_string();
                        println!("CoDLinux: New Wine Prefix: {}", self.wine_prefix);
                    }
                }
            });
        });
    }
}