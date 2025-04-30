//use std::thread;

use std::fs;
use std::thread;
use std::path::Path;
use std::time::Duration;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
// stairs xD
//use std::sync::atomic::{AtomicBool, Ordering};

use crate::utils::{moss_running, moss_capturing, notify, exec_command};

pub(crate) static ASSIST_MOSS: AtomicBool = AtomicBool::new(false);

pub(crate) fn capture() -> std::io::Result<Option<thread::JoinHandle<()>>>
{
    if !ASSIST_MOSS.load(Ordering::Relaxed) {
        return Ok(None);
    }
    if !moss_running()? {
        notify("Moss not running!", 6000).unwrap();
        return Ok(None);
    }

    if !moss_capturing()? {
        notify("Moss not capturing!", 6000).unwrap();
        return Ok(None);
    }

    println!("Starting screenshot capture process");

    println!("creating thread");
    let handle = thread::spawn(move || {
        let tmp_dir = Path::new("/tmp/codlinux_ss");
        if !tmp_dir.exists() {
            fs::create_dir(tmp_dir).unwrap();
        }
        //let capturing = Arc::new(AtomicBool::new(true));
        //let capturing_clone = Arc::clone(&capturing);
        
        let mut count = 0;
        while moss_capturing().unwrap()
        {
            let ss_path = tmp_dir.join(format!("{:03}.JPG", count));
            println!("Capturing screenshot to {}", ss_path.to_str().unwrap());
            let output = Command::new("scrot")
                .arg("-z")
                .arg(ss_path.to_str().unwrap())
                .output()
                .expect("Failed to execute scrot");
            if !output.status.success() {
                eprintln!("Failed to capture screenshot: {:?}", output.stderr);
            }
            count += 1;
            thread::sleep(Duration::from_secs(15));
        }
        thread::sleep(Duration::from_secs(6)); // make sure moss has finished
        
        let path_ = format!("{}/Desktop/MOSS", std::env::var("HOME").unwrap());
        let moss_dir = Path::new(&path_);
        println!("{}", moss_dir.to_str().unwrap());
        let mut zip_files: Vec<_> = fs::read_dir(moss_dir).unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "zip"))
            .collect();
        
        zip_files.sort_by_key(|entry| entry.metadata().unwrap().modified().unwrap());
        if let Some(last_zip) = zip_files.last() {
            let last_zip_path = last_zip.path();
            println!("Updating zip file: {}", last_zip_path.to_str().unwrap());
            let del_cmd = format!("zip -d {} *.JPG", last_zip_path.to_str().unwrap());
            exec_command(&del_cmd).unwrap();
            thread::sleep(Duration::from_secs(1));
            /*let output = Command::new("zip")
                .arg("-d")
                .arg(last_zip_path.to_str().unwrap())
                .arg("*.JPG")
                .output()
                .expect("Failed to execute zip");
            if !output.status.success() {
                eprintln!("Failed to delete files from zip: {:?} {:?}", output.stdout, output.stderr);
            }*/
            let add_cmd = format!("zip -rv {} ./*", last_zip_path.to_str().unwrap());
            let output = Command::new("bash")
                .arg("-c")
                .arg(&add_cmd)
                .current_dir("/tmp/codlinux_ss")
                .output().expect("Failed to execute  command");
            if !output.status.success() {
                eprintln!("Error adding files: stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            }
            //exec_command(&add_cmd).unwrap();
            /*let output = Command::new("zip")
                .arg("-rv")
                .arg(last_zip_path.to_str().unwrap())
                .arg("/tmp/codlinux_ss/*")
                .output()
                .expect("Failed to execute zip");
            if !output.status.success() {
                eprintln!("Failed to delete files from zip: {:?} {:?}", output.stdout, output.stderr);
            }*/
            */
            exec_command("rm -rf /tmp/codlinux_ss/*").unwrap();
        }
        else {
            eprintln!("No Moss zip files found to update");
        }
    });
    
    println!("Screenshot capture thread spawned");
    Ok(Some(handle))
}
