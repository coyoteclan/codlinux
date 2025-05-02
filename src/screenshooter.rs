//use std::thread;

use std::fs;
use std::thread;
use chrono::Utc;
use std::path::Path;
use std::time::Duration;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
// stairs xD
//use std::sync::atomic::{AtomicBool, Ordering};

use crate::utils::{moss_running, notify, exec_command, last_moss_file, GAME_RUNNING};

pub(crate) static ASSIST_MOSS: AtomicBool = AtomicBool::new(false);
pub(crate) static CAPTURE: AtomicBool = AtomicBool::new(false);

pub(crate) fn capture() -> std::io::Result<Option<thread::JoinHandle<()>>>
{
    if !ASSIST_MOSS.load(Ordering::Relaxed) {
        return Ok(None);
    }

    println!("Starting screenshot capture process");
    exec_command("rm -rf /tmp/codlinux_ss/*").unwrap();

    println!("creating thread");
    let handle = thread::spawn(move || {
        while !CAPTURE.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(2));
        }
        while !moss_running().unwrap() {
            notify("Open Moss!", 3000, true).unwrap();
            thread::sleep(Duration::from_secs(3));
        }
        while !GAME_RUNNING.load(Ordering::Relaxed) {
            notify("Game not running", 3000, true).unwrap();
            thread::sleep(Duration::from_secs(3));
        }

        let tmp_dir = Path::new("/tmp/codlinux_ss");
        if !tmp_dir.exists() {
            fs::create_dir(tmp_dir).unwrap();
        }
        thread::sleep(Duration::from_secs(3)); // wait for the game to open

        let mut count = 1;
        let mut last_ss_time = Utc::now();
        while CAPTURE.load(Ordering::Relaxed) && GAME_RUNNING.load(Ordering::Relaxed)
        {
            if Utc::now() - last_ss_time < chrono::Duration::seconds(10) {
                continue;
            }
            let ss_path = tmp_dir.join(format!("{:03}.JPG", count));
            println!("Capturing screenshot to {}", ss_path.to_str().unwrap());
            let output = Command::new("scrot")
                .arg("-z")
                .arg("-o")
                .arg("-u")
                .arg(ss_path.to_str().unwrap())
                .output()
                .expect("Failed to execute scrot");
            if !output.status.success() {
                eprintln!("Failed to capture screenshot: {:?}", output.stderr);
            }
            count += 1;
            last_ss_time = Utc::now();
            thread::sleep(Duration::from_secs(3));
        }
        // make sure moss has finished
        while moss_running().unwrap() {
            notify("Close Moss!", 2000, true).unwrap();
            thread::sleep(Duration::from_secs(3));
        }
        
        let last_zip = last_moss_file().expect("Couldn't get a zip file");
        println!("Updating zip file: {}", last_zip.to_str().unwrap());

        let del_cmd = format!("zip -d {} *.JPG", last_zip.to_str().unwrap());
        exec_command(&del_cmd).unwrap();
        thread::sleep(Duration::from_secs(1));

        let add_cmd = format!("zip -rv {} ./*", last_zip.to_str().unwrap());
        let output = Command::new("bash")
            .arg("-c")
            .arg(&add_cmd)
            .current_dir("/tmp/codlinux_ss")
            .output().expect("Failed to execute  command");
        if !output.status.success() {
            eprintln!("Error adding files: stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        }

        exec_command("mkdir /tmp/codlinux_ss/old").unwrap();
        exec_command("mv /tmp/codlinux_ss/*.JPG /tmp/codlinux_ss/old").unwrap();
        CAPTURE.store(false, Ordering::Relaxed);
        notify("Restart CoDLinux for capturing agian.", 5000, false).unwrap();
        std::process::exit(0);
    });
    
    println!("Screenshot capture thread spawned");
    Ok(Some(handle))
}
