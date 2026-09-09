mod wm;
mod config;
mod client;
mod x11;

fn main() {
    if let Err(err) = run() {
        eprintln!("simp-wm: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut wm = wm::WindowManager::new()?;
    wm.run()?;

    Ok(())
}
