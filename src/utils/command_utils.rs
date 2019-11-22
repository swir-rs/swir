use std::process::Command;

pub fn run_java_command(cmd:String) {
    info!("Running command bash {}", cmd);
    if !cmd.is_empty() {
        let mut o = Command::new("java")
            .arg("-jar")
            .arg(cmd)
            .spawn().unwrap();

        let r = o.try_wait();
        match r {
            Ok(Some(t)) => warn!("already exited  {}", t),
            Ok(None) => warn!("no status"),
            Err(e) => warn!("error {}", e)
        }
    };
}