fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Vesper Gate");
        res.set("FileDescription", "Vesper Gate - Ultra-Fast Local Reverse Proxy & Gateway");
        res.set("LegalCopyright", "Copyright (c) 2026 minseokk77");
        if let Err(e) = res.compile() {
            eprintln!("아이콘 리소스 컴파일 경고: {}", e);
        }
    }
}
