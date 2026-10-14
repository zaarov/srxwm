mod config;
mod wm;
mod x11;

fn main() {
    if let Err(error) = run() {
        eprintln!("simp-wm: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), wm::WmError> {
    let mut wm: wm::WindowManager = wm::WindowManager::new()?;
    wm.run()?;

    Ok(())
}
