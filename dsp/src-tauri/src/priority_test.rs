fn main() {
    let start = std::time::Instant::now();
    println!("Current priority class (before): {:?}", get_priority());
    
    set_high_priority();
    
    println!("Current priority class (after): {:?}", get_priority());
}

#[cfg(windows)]
fn get_priority() -> u32 {
    use std::os::windows::io::AsRawHandle;
    unsafe {
        winapi::um::processthreadsapi::GetPriorityClass(winapi::um::processthreadsapi::GetCurrentProcess())
    }
}

#[cfg(windows)]
fn set_high_priority() {
    unsafe {
        winapi::um::processthreadsapi::SetPriorityClass(
            winapi::um::processthreadsapi::GetCurrentProcess(),
            winapi::um::winbase::HIGH_PRIORITY_CLASS
        );
    }
}
