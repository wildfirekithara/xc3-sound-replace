use fnv::{FnvHashSet, FnvHasher};
use skyline::{hook, from_offset};
use std::collections::HashMap;
use crate::logging::dbg_println;

mod logging;

static mut REPLACEMENT_SET: Option<FnvHashSet<u32>> = None;
static mut NAME_LOOKUP: Option<HashMap<u32, String>> = None;

#[from_offset(0x0005dd7c)]
fn wwise_file_open_fallback(
    this: u64,
    file_name: u32,
    p3: u32,
    p4: *const u32,
    p5: *const i8,
    p6: u64,    
) -> u64;

#[hook(offset = 0x0006453c)]
unsafe fn wwise_file_open(
    this: u64,
    file_name: u32,
    p3: u32,
    p4: *const u32,
    p5: *const i8,
    p6: u64,
) -> u64 {

    let display_name = NAME_LOOKUP.as_ref().and_then(|s| s.get(&file_name));

    // The game will try to load the file from the File Package (.pck) archive, but it will fall back
    // to the rom:/sound directory if it's not found. By skipping the file lookup, we can force it to
    // load from the base directory instead of the archive.
    if REPLACEMENT_SET
        .as_ref()
        .and_then(|s| s.get(&file_name))
        .is_some()
    {
        if let Some(display_name) = display_name {
            dbg_println!("loading REPLACED {} - {}", file_name, display_name);
        } else {
            dbg_println!("loading REPLACED {}", file_name);
        }
    
        wwise_file_open_fallback(this, file_name, p3, p4, p5, p6)
    } else {
        if let Some(display_name) = display_name {
            dbg_println!("loading {} - {}", file_name, display_name);
        } else {
            dbg_println!("loading {}", file_name);
        }
    
        call_original!(this, file_name, p3, p4, p5, p6)
    }
}


#[from_offset(0x0005dcfc)]
fn wwise_direct_open_fallback(
    this: u64,
    p2: *const u8,
    p3: u32,
    p4: *const u32,
    p5: *const i8,
    p6: u64,
) -> u64;

#[hook(offset = 0x00064364)]
unsafe fn wwise_direct_open(
    this: u64,
    p2: *const u8,
    p3: u32,
    p4: *const u32,
    p5: *const i8,
    p6: u64,
) -> u64 {
    let file_name = std::ffi::CStr::from_ptr(p2).to_string_lossy();
    dbg_println!("loading direct {}", file_name);
    call_original!(this, p2, p3, p4, p5, p6)
}


#[skyline::main(name = "xc3_sound_replace")]
pub fn main() {
    println!("[XC3-SND] Loading...");

    let mut file_set = FnvHashSet::default();
    let sound_dir = std::fs::read_dir("rom:/sound/").expect("TODO");

    for sound_file in sound_dir {
        if let Ok(entry) = sound_file {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".wem") || name.ends_with(".bnk") {
                let id = name.split('.').next().unwrap();
                if let Ok(id) = id.parse() {
                    file_set.insert(id);
                }
            }
        }
    }

    if file_set.is_empty() {
        println!("[XC3-SND] No replacement files found, aborting.");
        return;
    }

    let mut name_lookup = HashMap::new();
    let lookup_file = include_str!("../lookup.csv");
    for line in lookup_file.lines() {
        let mut split = line.split(',');
        let id = split.next().unwrap();
        let name = split.next().unwrap();
        if let Ok(id) = id.parse() {
            name_lookup.insert(id, name.to_string());
        }
    }

    unsafe {
        REPLACEMENT_SET = Some(file_set);
        NAME_LOOKUP = Some(name_lookup);
    };

    println!("[XC3-SND] Installing hooks");
    skyline::install_hooks!(wwise_file_open, wwise_direct_open);

    println!("[XC3-SND] Loaded!");
}
