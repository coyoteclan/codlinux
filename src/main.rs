static VERSION: &str = env!("CARGO_PKG_VERSION");

static GNAME_STYLE: &str = "font-family=\"Ubuntu\" font-weight=\"bold\" font-size=\"xx-large\"";

use relm4::{
    factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque}, gtk, Component, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt, Worker, WorkerController
};
use gtk::Orientation;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt, PopoverExt, GridExt, EditableExt};
use util::my_exe_path;

mod util;
mod updater;

use std::{io, env, fs::create_dir_all, path::PathBuf, collections::BTreeMap};

//use relm4_icons_build;
//use relm4_icons;

//mod icon_names {
//    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
//}


// Structs & Enums
#[derive(Debug, Clone)]
struct GameInfo {
    name: String,
    version: String,
    path: PathBuf,
    wineprefix: String,
    envars: String,
    args: String,
}

#[derive(Debug, Clone)]
enum GameMsg {}

#[derive(Debug)]
enum GameOutput {
    Launched(DynamicIndex),
    Removed(DynamicIndex),
    Edited(DynamicIndex, String, String, String),
    Remembered(DynamicIndex),
}

struct App {
    games: FactoryVecDeque<GameInfo>,
    scanner: WorkerController<Scanner>,
    launcher: WorkerController<GameLauncher>,
}

#[derive(Debug)]
enum AppMsg {
    AddGames(Vec<GameInfo>),
    AddGame,
    RefreshGames,
    RemoveGame(DynamicIndex),
    LaunchGame(DynamicIndex),
    ExitGame,
    UpdateGame(DynamicIndex, String, String, String),
    ShowUpdater,
    RememberGame(DynamicIndex),
}

struct Scanner;

impl Worker for Scanner {
    type Init = ();
    type Input = ();
    type Output = Vec<GameInfo>;

    fn init(_: (), _sender: ComponentSender<Self>) -> Self { Scanner }

    fn update(&mut self, _: (), sender: ComponentSender<Self>)
    {
        let games = scan_games().unwrap_or_default();
        sender.output(games).unwrap();
    }
}

struct GameLauncher;

impl Worker for GameLauncher {
    type Init = ();
    type Input = GameInfo;
    type Output = ();

    fn init(_: (), _sender: ComponentSender<Self>) -> Self { GameLauncher }

    fn update(&mut self, game: Self::Input, sender: ComponentSender<Self>)
    {
        // ChatGPT
        // run in a `catch_unwind` so we never let a panic escape
        // prevents the case where the app keeps running in background if game process exits with an error
        let result = std::panic::catch_unwind(|| {
            if let Err(err) = util::launch_game(&game) {
                eprintln!("⚠️ Game launch failed for `{}`: {}", game.name, err);
            }
        });

        if result.is_err() {
            eprintln!("⚠️ Panic while launching `{}`", game.name);
        }

        // and *always* let App know we're done
        sender.output(()).expect("Failed to send ExitGame");
    }
}

#[relm4::factory]
impl FactoryComponent for GameInfo {
    type Init = GameInfo;
    type Input = GameMsg;
    type Output = GameOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        root = gtk::Box {
            set_orientation: Orientation::Horizontal,
            set_spacing: 6,
            set_align: gtk::Align::Center,

            gtk::Button {
                connect_clicked[sender, index] => move |_| {
                    sender.output(GameOutput::Launched(index.clone())).unwrap();
                },
                set_size_request: (300, 150),

                gtk::Box {
                    set_orientation: Orientation::Vertical,
                    set_expand: false,
                    //set_align: gtk::Align::Center,

                    gtk::Box {
                        set_orientation: Orientation::Horizontal,
                        set_expand: true,
                        set_align: gtk::Align::Center,

                        gtk::Label {
                            set_markup: &format!("<span {}>{}</span>", GNAME_STYLE, &self.name),
                        },
                    },

                    gtk::Box {
                        set_orientation: Orientation::Horizontal,
                        set_expand: false,
                        set_align: gtk::Align::End,

                        gtk::Label {
                            set_text: &format!("v{}", self.version),
                        }
                    }
                },
            },

            gtk::Box {
                set_orientation: Orientation::Vertical,
                set_spacing: 5,
                set_expand: false,
                set_align: gtk::Align::Start,

                gtk::Button {
                    set_icon_name: "list-remove",
                    connect_clicked[sender, index] => move |_| {
                        sender.output(GameOutput::Removed(index.clone())).unwrap();
                    },
                    set_size_request: (32,32)
                },

                gtk::MenuButton {
                    set_icon_name: "document-edit",
                    set_direction: gtk::ArrowType::Right,

                    #[wrap(Some)]
                    set_popover: popover = &gtk::Popover {
                        set_position: gtk::PositionType::Right,
                        set_autohide: true,

                        gtk::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,

                            gtk::Grid {
                                set_row_spacing: 6,
                                set_column_spacing: 12,
                                set_margin_all: 12,
                                set_column_homogeneous: false,
                                set_row_homogeneous: false,

                                attach[0,0,1,1] = &gtk::Label {
                                    set_markup: "<b>Wine Prefix</b>",
                                    set_halign: gtk::Align::Start,
                                },
                                #[name = "wine_prefix_entry"]
                                attach[1, 0, 1, 1] = &gtk::Entry {
                                    set_text: &self.wineprefix,
                                    set_hexpand: true,
                                },

                                attach[0, 1, 1, 1] = &gtk::Label {
                                    set_markup: "<b>Env Vars</b>",
                                    set_halign: gtk::Align::Start,
                                },
                                #[name = "envars_entry"]
                                attach[1, 1, 1, 1] = &gtk::Entry {
                                    set_text: &self.envars,
                                    set_hexpand: true,
                                },

                                attach[0, 2, 1, 1] = &gtk::Label {
                                    set_markup: "<b>Args</b>",
                                    set_halign: gtk::Align::Start,
                                },
                                #[name = "args_entry"]
                                attach[1, 2, 1, 1] = &gtk::Entry {
                                    set_text: &self.args,
                                    set_hexpand: true,
                                },
                            },

                            gtk::Button {
                                set_label: "Remember Game",
                                connect_clicked[sender, index] => move |_| {
                                    sender.output(GameOutput::Remembered(index.clone())).unwrap();
                                },
                            },

                            gtk::Button {
                                set_label: "Save",
                                connect_clicked[sender, index, wine_prefix_entry, envars_entry, args_entry, popover] => move |_| {
                                    let wp = wine_prefix_entry.text().trim().to_string();
                                    let ev = envars_entry.text().trim().to_string();
                                    let ag = args_entry.text().trim().to_string();
                                    sender.output(GameOutput::Edited(index.clone(), wp, ev, ag)).unwrap();

                                    popover.popdown();
                                }
                            }
                        },
                    }
                },
            },
        }
    }

    fn init_model(info: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> GameInfo
    {
        info
    }
}

#[relm4::component]
impl Component for App {
    type Init = Vec<GameInfo>;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_title: Some("CoDLinux"),
            set_default_size: (400, 300),

            gtk::Box {
                set_orientation: Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 8,

                gtk::Box {
                    set_orientation: Orientation::Horizontal,
                    set_align: gtk::Align::End,
                    set_spacing: 5,

                    gtk::Button {
                        set_icon_name: "view-refresh",
                        set_expand: false,
                        connect_clicked => AppMsg::RefreshGames,
                    },

                    gtk::MenuButton {
                        set_icon_name: "view-more",
                        set_direction: gtk::ArrowType::Down,

                        #[wrap(Some)]
                        set_popover: more_popover = &gtk::Popover {
                            set_position: gtk::PositionType::Bottom,
                            set_autohide: true,
                            set_cascade_popdown: false,

                            gtk::Box {
                                set_orientation: Orientation::Vertical,
                                set_spacing: 5,

                                gtk::Button {
                                    set_label: "Add Dummy Game",
                                    connect_clicked => AppMsg::AddGame,
                                },
                                gtk::Button {
                                    set_label: "Check for Updates",
                                    connect_clicked[sender, more_popover] => move |_| {
                                        more_popover.popdown();
                                        sender.input(AppMsg::ShowUpdater);
                                    },
                                },
                                gtk::MenuButton {
                                    set_label: "About",
                                    set_direction: gtk::ArrowType::Right,

                                    #[wrap(Some)]
                                    set_popover: about_popover = &gtk::Popover {
                                        set_position: gtk::PositionType::Right,

                                        gtk::Box {
                                            set_orientation: Orientation::Vertical,
                                            set_width_request: 200,
                                            //set_height_request: 200,
                                            set_align: gtk::Align::Center,
                                            set_spacing: 6,
                                            set_margin_all: 4,

                                            gtk::Image {
                                                set_icon_name: Some("codlinux"),
                                                set_icon_size: gtk::IconSize::Large,
                                            },
                                            gtk::Label {
                                                set_markup: &format!("<b>CoDLinux v{}</b>", &VERSION)
                                            },
                                            gtk::Label {
                                                set_text: "CoD 1/UO client helper"
                                            },
                                            gtk::Label {
                                                set_markup: "<a href=\"https://github.com/coyoteclan/codlinux\">Repo</a> | <a href=\"https://discord.gg/kSrXbj9shh\">Discord</a>"
                                            },
                                            gtk::Label {
                                                set_markup: "<small>© 2025 Kazam</small>"
                                            },
                                            gtk::Label {
                                                set_markup: "<small>This program comes with absolutely no warranty.</small>"
                                            },
                                            gtk::Label {
                                                set_markup: "<small>See the <a href=\"https://www.gnu.org/licenses/gpl-3.0.html#license-text\">GNU General Public License, version 3 or later</a> for details.</small>"
                                            }
                                        }
                                    }
                                },

                                gtk::Button {
                                    set_label: "Close",
                                    connect_clicked[more_popover] => move |_| {
                                        more_popover.popdown();
                                    }
                                }
                            }
                        }
                    },
                },

                #[local_ref]
                games_box -> gtk::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 5,
                },
            }
        }
    }

    fn init(games_list: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self>
    {
        let games = FactoryVecDeque::builder()
        .launch_default()
        .forward(sender.input_sender(), |msg| match msg {
            GameOutput::Launched(index) => AppMsg::LaunchGame(index),
            GameOutput::Removed(index) => AppMsg::RemoveGame(index),
            GameOutput::Edited(index, wp, ev, ag) => AppMsg::UpdateGame(index, wp, ev, ag),
            GameOutput::Remembered(index) => AppMsg::RememberGame(index),
        });

        let scanner = Scanner::builder()
            .detach_worker(())
            .forward(sender.input_sender(), AppMsg::AddGames);

        let launcher = GameLauncher::builder()
            .detach_worker(())
            .forward(sender.input_sender(), |_| AppMsg::ExitGame);

        let model = App { games, scanner, launcher };
        let games_box = model.games.widget();
        let widgets = view_output!();

        sender.input_sender().send(AppMsg::AddGames(games_list)).unwrap();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root)
    {
        let mut games_guard = self.games.guard();
        match msg {
            AppMsg::AddGames(list) => {
                for game in list {
                    games_guard.push_back(game);
                }
            }
            AppMsg::AddGame => {
                games_guard.push_back(GameInfo {
                    name: "Dummy Game".to_string(),
                    version: "1.0".to_string(),
                    path: std::path::PathBuf::from("/path/to/game"),
                    wineprefix: String::new(),
                    envars: String::new(),
                    args: String::new(),
                });
            }
            AppMsg::RefreshGames => {
                games_guard.clear();
                self.scanner.emit(());
            }
            AppMsg::RemoveGame(index) => {
                let _ = games_guard.remove(index.current_index());
            }
            AppMsg::LaunchGame(index) => {
                println!("Launch: {:?}", index);
                let game = games_guard.get(index.current_index()).unwrap();

                println!("{:#?}", game);
                if &game.name == "Dummy Game" { return; }

                root.set_visible(false);
                self.launcher.emit(game.clone());
                //util::launch_game(game).unwrap();
                //root.set_visible(true);
            }
            AppMsg::ExitGame => {
                root.set_visible(true);
            }
            AppMsg::UpdateGame(index, wp, ev, ag) => {
                if let Some(game) = games_guard.get_mut(index.current_index()) {
                    game.wineprefix = wp;
                    game.envars = ev;
                    game.args = ag;

                    let mut game_config: BTreeMap<String, String> = BTreeMap::new();
                    game_config.insert("wine_prefix".to_string(), game.wineprefix.clone());
                    game_config.insert("envars".to_string(), game.envars.clone());
                    game_config.insert("args".to_string(), game.args.clone());
                    util::save_game_config(&game.name, &game_config).unwrap();
                }
            }
            AppMsg::ShowUpdater => {
                updater::show_update_window(root.application().unwrap());
            }
            AppMsg::RememberGame(index) => {
                if let Some(game) = games_guard.get_mut(index.current_index()) {
                    util::save_setting("saved_game", &game.name).unwrap();
                }
            }
        }

        /*if util::GAME_RUNNING.load(Ordering::Relaxed) {
            root.set_visible(false);
        }
        else {
            root.set_visible(true);
        }*/
    }
}

fn load_game_settings(mut game: GameInfo) -> io::Result<GameInfo>
{
    let cfg_file = util::my_exe_path().unwrap().join(format!("codlinux_conf/{}.cfg", &game.name));
    if cfg_file.exists() {
        let cfg = util::get_game_config(&game.name).unwrap();
        game.wineprefix = if let Some(wp) = cfg.get("wine_prefix") {
            wp.to_string()
        } else {
            String::new()
        };
        game.envars = if let Some(ev) = cfg.get("envars") {
            ev.to_string()
        } else {
            String::new()
        };
        game.args = if let Some(ag) = cfg.get("args") {
            ag.to_string()
        } else {
            String::new()
        };
    }
    Ok(game.clone())
}

fn scan_games() -> Result<Vec<GameInfo>, String>
{
    let cfgdir = my_exe_path().unwrap().join("codlinux_conf");
    if !cfgdir.exists() {
        println!("{:#?} does not exist", cfgdir);
        create_dir_all(cfgdir).unwrap();
    }

    let executables = util::get_exes().unwrap_or_default();
    let games: Vec<GameInfo> = executables.into_iter().flat_map(|exe| {
        let name_version: Vec<(String, String)> = util::name_version_info(&exe).unwrap();
        //let exe_clone = exe.clone();
        name_version.into_iter().map(move |(name, version)| {
            let cfg_file = util::my_exe_path().unwrap().join(format!("codlinux_conf/{}.cfg", &name));
            let mut wine_prefix = String::new();
            let mut env_vars = String::new();
            let mut args_ = String::new();
            if cfg_file.exists() {
                let cfg = util::get_game_config(&name).unwrap();
                wine_prefix = cfg.get("wine_prefix").unwrap().to_string();
                env_vars = cfg.get("envars").unwrap().to_string();
                args_ = cfg.get("args").unwrap().to_string();
            }
            GameInfo {
                name,
                version,
                path: exe.clone(),
                wineprefix: wine_prefix,
                envars: env_vars,
                args: args_,
            }
        })
    }).collect();

    Ok(games)
}

fn main() -> io::Result<()>
{
    println!("CoDLinux v{}", &VERSION);
    create_dir_all(my_exe_path()?.join("codlinux_conf"))?;
    if util::load_setting("default_wine_prefix").unwrap().is_empty() {
        util::save_setting("default_wine_prefix", "$HOME/.wine").unwrap();
    }
    util::extract_icon()?;

    let resolution = util::get_display_mode();
    if let Some((width, height, rate)) = resolution {
        println!("CoDLinux: Display resolution: {}x{} {} Hz", width, height, rate);
    }
    else {
        println!("CoDLinux: Unable to get display resolution.");
    }
    println!("{:#?}", util::DISPLAY_OUTPUT);
    if true {
        //std::process::exit(0);
    }

    println!("CoDLinux: Looking for game executables...");
    let mut uo = false;
    let mut cod1 = false;
    let mut iw1x = false;
    let mut t1x = false;
    let games = scan_games().unwrap();

    if games.is_empty() {
        println!("CoDLinux: No game executables found.");
    }

    for game in &games {
        match game.version.as_str() {
            "1.1" => cod1 = true,
            "1.51" => uo = true,
            _ => ()
        }
        match game.name.as_str() {
            "IW1X" => iw1x = true,
            "T1X" => t1x = true,
            _ => ()
        }
    }

    if cod1 || uo {
        util::create_desktop_file(&uo, my_exe_path().unwrap().to_str().unwrap())?;
    }
    if t1x && uo {
        util::reg_uri_scheme("t1x")?;
    }
    else if iw1x && cod1 {
        util::reg_uri_scheme("iw1x")?;
    }

    let mut launched = false;
    let mut args: Vec<String> = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(first_arg) = args.get(0) {
        if first_arg.starts_with("iw1x://") || first_arg.starts_with("t1x://") {
            let _match = if first_arg.starts_with("iw1x://") { "iw1x://" } else { "t1x://" };
            let stripped = first_arg.trim_start_matches(_match);
            let parts: Vec<&str> = stripped.split(':').collect();
            let ip = parts.get(0).unwrap_or(&"127.0.0.1");
            let port = parts.get(1).unwrap_or(&"28960");

            let scheme = first_arg.split(":").nth(0).unwrap().to_uppercase(); // IW1X or T1X

            args[0] = format!("+connect {}:{}", ip, port);
            args[1] = String::from("+set r_ignorehwgamma 1");

            let args_str = args.join(" ");
            if iw1x || t1x {
                for game in &games {
                    if game.name == scheme {
                        util::notify(&format!("Launching {scheme}..."), 2000, false).unwrap();
                        let mut game = load_game_settings(game.clone()).unwrap(); // TODO check if there's a better way to do this
                        game.args = format!("{} {}", &game.args, &args_str);
                        util::launch_game(&game)?;
                        launched = true;
                    }
                }
            }
        }
    }

    if !launched {
        let args_str = args.join(" ");
        let saved_game = util::load_setting("saved_game").unwrap();
        if !saved_game.is_empty() {
            for game in &games {
                if game.name == saved_game {
                    let mut game = load_game_settings(game.clone()).unwrap(); // TODO check if there's a better way to do this

                    game.args = format!("{} {}", &game.args, &args_str);

                    util::launch_game(&game)?;
                    launched = true;
                }
            }
        }
    }

    if !launched {
        let app = RelmApp::new("wolfpack.kazam.codlinux");
        app.run::<App>(games);
    }

    Ok(())
}
