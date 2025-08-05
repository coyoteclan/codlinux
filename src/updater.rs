use futures_util::{FutureExt, StreamExt};
use relm4::{gtk::{self, Application}, RelmWidgetExt, tokio::fs, tokio::io::AsyncWriteExt, Component, ComponentController, ComponentParts, ComponentSender};
use gtk::prelude::*;

use std::{os::unix::fs::PermissionsExt, time::Duration};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

pub(crate) fn show_update_window(app: Application)
{
    //RelmApp::new("wolfpack.kazam.codlinux-updater").run::<Updater>("Updater".into());
    let builder = Updater::builder();
    app.add_window(&builder.root);
    builder.root.set_visible(true);
    builder.root.set_title("Updater".into());
    builder.launch("Updater".into()).detach_runtime();
}

#[derive(Default)]
pub struct Updater {
    /// Tracks progress status
    checking: bool,
    downloading: bool,

    /// Contains output of a completed task.
    task: Option<CmdOut>,
}

pub struct Widgets {
    checkbutton: gtk::Button,
    updatebutton: gtk::Button,
    cancelbutton: gtk::Button,
    okbutton: gtk::Button,
    label: gtk::Label,
    whatsnew: gtk::Label,
    changelog_container: gtk::ScrolledWindow,
    sizelabel: gtk::Label,
    spinner: gtk::Spinner,
    progress: gtk::ProgressBar,
}

#[derive(Debug)]
pub enum Input {
    CheckUpdate,
    Download,
    Exit,
}

#[derive(Debug)]
pub enum Output {
    //Clicked(u32),
}

#[derive(Debug)]
pub enum CmdOut {
    Checking,
    /// The final output of the command.
    Checked(Result<(bool, String), String>),
    Download,
    /// Progress update from a command.
    Progress(String, f32),
    /// The final output of the command.
    Finished(Result<String, String>),
}

impl Component for Updater {
    type Init = String;
    type Input = Input;
    type Output = Output;
    type CommandOutput = CmdOut;
    type Widgets = Widgets;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .width_request(350)
            .height_request(150)
            .build()
    }

    fn init(
        _args: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        relm4::view! {
            container = gtk::Box {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_spacing: 6,
                set_margin_all: 4,
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,

                    append: label = &gtk::Label {
                        set_label: "",
                        set_hexpand: true,
                    },

                    append: sizelabel = &gtk::Label {
                        set_label: "0.0/0.0 MB",
                        set_visible: false,
                    }
                },

                append: spinner = &gtk::Spinner {
                    set_visible: false,
                },

                #[name = "changelog_container"]
                gtk::ScrolledWindow {
                    set_width_request: 250,
                    set_height_request: 75,
                    set_visible: false,
                    set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),

                    //append: whatsnew = &gtk::Label {
                    #[name = "whatsnew"]
                    gtk::Label {
                        set_markup: "",
                        set_hexpand: true,
                        set_align: gtk::Align::Start,
                        set_margin_all: 4,
                    },
                },

                append: progress = &gtk::ProgressBar {
                    set_width_request: 200,
                    set_visible: false,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 16,
                    set_align: gtk::Align::Center,

                    append: checkbutton = &gtk::Button {
                        set_label: "Check",
                        set_hexpand: false,
                        connect_clicked => Input::CheckUpdate,
                    },

                    append: updatebutton = &gtk::Button {
                        set_label: "Update",
                        set_hexpand: false,
                        set_visible: false,
                        connect_clicked => Input::Download,
                    },

                    append: cancelbutton = &gtk::Button {
                        set_label: "Cancel",
                        set_hexpand: false,
                        set_visible: false,
                        connect_clicked => Input::Exit,
                    },

                    append: okbutton = &gtk::Button {
                        set_label: "OK",
                        set_hexpand: false,
                        set_visible: false,
                        connect_clicked => Input::Exit,
                    }
                },
            }
        }

        root.set_child(Some(&container));

        ComponentParts {
            model: Updater::default(),
            widgets: Widgets {
                label,
                whatsnew,
                changelog_container,
                sizelabel,
                checkbutton,
                updatebutton,
                cancelbutton,
                okbutton,
                spinner,
                progress,
            },
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            Input::CheckUpdate => {
                self.checking = true;

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            out.send(CmdOut::Checking).unwrap();
                            let result = fetch_latest_release().await.unwrap();

                            let compile_time = DateTime::parse_from_rfc3339(
                                compile_time::datetime_str!()
                            ).unwrap().with_timezone(&Utc);

                            let difference = result.published_at - compile_time;
                            let body = result.body.clone();
                            let mut changelog = String::new();
                            let lines = body.lines().collect::<Vec<_>>();
                            for line in &lines[1..] {
                                // trim end to avoid spurious `\r`
                                let line = line.trim_end();

                                if let Some(text) = line.strip_prefix("## ") {
                                    changelog.push_str(&format!("<b>{}</b>\n\n", text));
                                }
                                else if let Some(item) = line.strip_prefix("- ") {
                                    let item = process_inline_code(item);
                                    changelog.push_str(&format!("• {}\n", item));
                                }
                                else if line.is_empty() {
                                    changelog.push('\n');
                                }
                                else {
                                    let text = process_inline_code(line);
                                    changelog.push_str(&format!("{}\n", text));
                                }
                            }
                            out.send(CmdOut::Checked(
                                Ok((difference > chrono::Duration::minutes(5), changelog))
                            )).unwrap();
                        })
                            .drop_on_shutdown()
                            .boxed()
                });
            }
            Input::Download => {
                self.downloading = true;

                sender.command(|out, shutdown| {
                    shutdown
                    // Performs this operation until a shutdown is triggered
                    .register(async move {
                        out.send(CmdOut::Download).unwrap();
                        // Must catch most of the errors here
                        // Entering golang mode xD
                        let result = match fetch_latest_release().await {
                            Ok(r) => r,
                            Err(e) => {
                                out.send(CmdOut::Finished(Err(format!("Release fetch failed: {}", e)))).unwrap();
                                return;
                            }
                        };
                        let asset = result.assets.get(0);
                        let url = if let Some(a) = asset {
                            a.browser_download_url.clone()
                        }
                        else {
                            out.send(CmdOut::Finished(Err("No downloadable assets in release.".into()))).unwrap();
                            return;
                        };
                        let client = match reqwest::Client::builder()
                            .timeout(Duration::from_secs(60))
                            .build() {
                                Ok(c) => c,
                                Err(e) => {
                                    out.send(CmdOut::Finished(Err(
                                        format!("HTTP client init failed: {}", e)
                                    ))).unwrap();
                                  return;
                              }
                        };

                        let resp = match client
                            .get(&url)
                            .send()
                            .await
                            .and_then(|r| r.error_for_status())
                        {
                            Ok(r) => r,
                            Err(e) => {
                                let msg = if e.is_timeout() { "download timed out" } else { "download failed" };
                                out.send(CmdOut::Finished(Err(format!("{}: {}", msg, e)))).unwrap();
                                return;
                            }
                        };
                        let total: f32 = asset.unwrap().size.clone();

                        out.send(CmdOut::Progress(
                            format!("{:.2}/{:.2} MB", total / (1024.0 * 1024.0), 1.0), 0.0)
                        ).unwrap();

                        let mut file = match fs::File::create("codlinux_new").await {
                            Ok(f) => f,
                            Err(e) => {
                                out.send(CmdOut::Finished(Err(format!("Cannot create file: {}", e)))).unwrap();
                                return;
                            }
                        };
                        let mut stream = resp.bytes_stream();
                        let mut downloaded = 0u64;

                        while let Some(chunk) = stream.next().await {
                            let bytes = match chunk {
                                Ok(c) => c,
                                Err(e) => {
                                    out.send(CmdOut::Finished(Err(format!("Network chunk error: {}", e)))).unwrap();
                                    return;
                                }
                            };
                            if let Err(e) = file.write_all(&bytes).await {
                                out.send(CmdOut::Finished(Err(format!("Write failed: {}", e)))).unwrap();
                                return;
                            }
                            downloaded = downloaded.saturating_add(bytes.len() as u64);

                            let frac = if total > 0f32 {
                                downloaded as f32 / total as f32
                            } else {
                                0.0
                            };
                            let hdl = downloaded as f32 / (1024.0 * 1024.0);
                            let htotal = total / (1024.0 * 1024.0);

                            out.send(CmdOut::Progress(
                                format!("{:.2}/{:.2} MB", hdl, htotal), frac)
                            ).unwrap();
                        }

                        let codlinux = crate::util::my_exe_path().unwrap().join("codlinux");
                        if let Err(e) = {
                            let _ = match fs::remove_file(&codlinux).await {
                                Ok(_) => (),
                                Err(e) => {
                                    out.send(CmdOut::Finished(Err(
                                        format!("Failed to remove old file: {}", e)
                                    ))).unwrap();
                                }
                            };

                            fs::rename("codlinux_new", &codlinux).await.unwrap();

                            let mut perms = fs::metadata(&codlinux).await.unwrap().permissions();
                            perms.set_mode(perms.mode() | 0o0100);
                            fs::set_permissions(&codlinux, perms).await
                        } {
                            out.send(CmdOut::Finished(Err(format!("File install error: {}", e)))).unwrap();
                            return;
                        }

                        out.send(CmdOut::Finished(Ok("Success!".into()))).unwrap();
                    })
                    // Perform task until a shutdown interrupts it
                    .drop_on_shutdown()
                    // Wrap into a `Pin<Box<Future>>` for return
                    .boxed()
                });
            }
            Input::Exit => {
                root.destroy();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let CmdOut::Checked(_) = message {
            self.checking = false;
        }

        self.task = Some(message);
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        //widgets.updatebutton.set_sensitive(!self.checking);

        if let Some(ref progress) = self.task {
            match progress {
                CmdOut::Checking => {
                    widgets.checkbutton.set_visible(false);
                    widgets.label.set_label("Checking for update...");
                    widgets.spinner.set_visible(true);
                    widgets.spinner.set_spinning(true);
                }
                CmdOut::Checked(result) => {
                    widgets.spinner.set_visible(false);
                    widgets.checkbutton.set_visible(false);
                    match result {
                        Ok((check, changelog)) => {
                            if *check {
                                widgets.label.set_label("What's new?");
                                widgets.whatsnew.set_markup(&changelog);
                                widgets.changelog_container.set_visible(true);
                                widgets.whatsnew.set_visible(true);
                                widgets.updatebutton.set_visible(true);
                                widgets.cancelbutton.set_visible(true);
                            }
                            else {
                                widgets.label.set_text("No update available.");
                                widgets.okbutton.set_visible(true);
                            }
                        }
                        Err(e) => widgets.label.set_label(&format!("Error: {e}")),
                    }
                }
                CmdOut::Download => {
                    widgets.updatebutton.set_visible(false);
                    widgets.cancelbutton.set_visible(false);
                    widgets.whatsnew.set_visible(false);
                    widgets.changelog_container.set_visible(false);
                    widgets.label.set_label("Downloading...");
                    widgets.label.set_halign(gtk::Align::Start);
                    widgets.sizelabel.set_halign(gtk::Align::End);
                    widgets.sizelabel.set_visible(true);
                    widgets.progress.set_visible(true);
                }
                CmdOut::Progress(p, f) => {
                    widgets.label.set_label(&format!("Downloading..."));
                    widgets.sizelabel.set_label(&p);
                    widgets.progress.set_fraction(*f as f64);
                }
                CmdOut::Finished(result) => {
                    widgets.progress.set_visible(false);
                    widgets.okbutton.set_visible(true);
                    widgets.sizelabel.set_visible(false);
                    match result {
                        Ok(s) => {
                            widgets.label.set_label(s);
                        }
                        Err(e) => {
                            widgets.label.set_label(&format!("Error: {e}"));
                        }
                    }
                }
            }
        }
    }
}

const OWNER: &str = "coyoteclan";
const REPO: &str = "codlinux";

#[derive(Debug, Deserialize)]
struct GHRelease {
    published_at: DateTime<Utc>,
    assets: Vec<ReleaseAsset>,
    body: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    browser_download_url: String,
    size: f32,
}

fn gh_client() -> Result<Client, reqwest::Error>
{
    let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .user_agent("codlinux-updater/1.0")
    .build()?;

    Ok(client)
}

async fn fetch_latest_release() ->Result<GHRelease, reqwest::Error>
{
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/releases/latest");
    let client = gh_client()?;

    let response = client.get(&url)
        .header("Accept", "application/vnd.github+json").send().await?.error_for_status()?;

    //if response.status().is_success() {
    let release: GHRelease = response.json().await?;
    Ok(release)
    //}
}

fn process_inline_code(s: &str) -> String {
    let mut out = String::new();
    let mut parts = s.split("``");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    let mut inside = true;
    for piece in parts {
        if inside {
            out.push_str("<tt>");
            out.push_str(piece);
            out.push_str("</tt>");
        } else {
            out.push_str(piece);
        }
        inside = !inside;
    }
    out
}
