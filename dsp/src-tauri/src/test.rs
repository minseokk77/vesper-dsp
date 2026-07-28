fn main() {
    let appdata = std::env::var("APPDATA").unwrap();
    let startup_dir = std::path::PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup");
    println!("Startup dir: {:?}", startup_dir);
}
