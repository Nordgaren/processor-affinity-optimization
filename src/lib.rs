#![allow(non_snake_case)]

use dll_proxy::proxy_dll;
use dll_proxy::winternals::{GetLastError, GetModuleFileNameA, GetModuleHandleA};
use std::ffi::c_void;
use std::io::{Error, ErrorKind};
use std::ops::{Deref, Index};
use std::{fs, path};
use std::path::Path;
use std::time::Duration;
use dll_proxy::utils::MAX_PATH;
use serde::Deserialize;

proxy_dll!("dinput8.dll");

const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;
const PROCESS_ALL_ACCESS: u32 = 0x000F0000 | 0x00100000 | 0xFFFF;
#[repr(C)]
#[derive(Copy, Clone)]
struct HANDLE(*const c_void);
impl HANDLE {
    pub fn is_valid(&self) -> bool {
        self.0 != 0 as _ && self.0 != -1 as _
    }
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub unsafe fn new(handle: HANDLE) -> OwnedHandle {
        Self(handle)
    }
}

impl Deref for OwnedHandle {
    type Target = HANDLE;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.is_valid() {
            unsafe { CloseHandle(self.0) }
        }
    }
}

#[link(name = "kernel32", kind = "raw-dylib")]
extern "system" {
    pub fn AllocConsole() -> u32;
    pub fn AttachConsole(dwProcessId: u32) -> u32;
    pub fn CloseHandle(object: HANDLE);
    pub fn GetCurrentProcessId() -> u32;
    pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: bool, dwProcessId: u32) -> HANDLE;
    pub fn GetProcessAffinityMask(
        hProcess: HANDLE,
        lpProcessAffinityMask: *mut usize,
        lpSystemAffinityMask: *mut usize,
    ) -> bool;
    pub fn SetProcessAffinityMask(
        hProcess: HANDLE,
        lpProcessAffinityMask: usize,
    ) -> bool;
}

#[derive(Deserialize)]
pub struct Config {
    pub delay: f64,
    pub exclude: Vec<u64>,
}

#[no_mangle]
#[allow(unused)]
pub extern "stdcall" fn DllMain(hinstDLL: usize, dwReason: u32, lpReserved: *mut usize) -> i32 {
    match dwReason {
        DLL_PROCESS_ATTACH => unsafe {
            #[cfg(feature = "Console")]
            {
                AllocConsole();
                AttachConsole(u32::MAX);
            }
            let path = match init_proxy(hinstDLL) {
                Ok(p) => p,
                Err(e) => panic!("Could not proxy dll: {e}"),
            };

            let config = read_config_file(hinstDLL).expect("Could not read config");


            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs_f64(config.delay));
                println!("Changing affinity");
                set_processor_affinity(get_exclude_mask(config.exclude)).expect("Could not set processor affinity");
            });
            1
        },
        DLL_PROCESS_DETACH => 1,
        _ => 0,
    }
}

fn get_exclude_mask(exclude: Vec<u64>) -> usize {
    let mut mask = 0;
    for p in exclude {
        mask |= 1 << p
    }

    mask
}

const CONFIG_PATH: &str = "affinity.toml";

unsafe fn get_file_name(hinstDLL: usize) -> String {
    let mut buffer = [0u8; MAX_PATH + 1];
    let name_size = GetModuleFileNameA(hinstDLL, buffer.as_mut_ptr(), buffer.len() as u32) as usize;
    let name = &buffer[..name_size];
    let name_str = std::str::from_utf8(name).expect("Could not parse name from GetModuleFileNameA");
    name_str.to_string()
}

fn read_config_file(hinstDLL: usize) -> std::io::Result<Config> {
    let name = unsafe { get_file_name(hinstDLL) };
    let path = Path::new(&name);
    let working_dir = path.parent().unwrap().to_str().unwrap();
    let f = fs::read_to_string(format!("{working_dir}/{CONFIG_PATH}")).expect("Could not read string");
    toml::from_str(&f)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))
}

fn set_processor_affinity(exclude: usize) -> std::io::Result<()> {
    let pid = unsafe { GetCurrentProcessId() };

    let process_handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, true, pid) };
    let process_handle = unsafe { OwnedHandle::new(process_handle) };
    if !process_handle.is_valid() {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            format!("Unable to open process. Last Error: {:X}", unsafe {
                GetLastError()
            }),
        ));
    }

    let mut process_affinity_mask = 0;
    let mut system_affinity_mask = 0;

    if !unsafe { GetProcessAffinityMask(*process_handle, &mut process_affinity_mask, &mut system_affinity_mask) } {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            format!("Unable to get process affinity mask. Last Error: {:X}", unsafe {
                GetLastError()
            }),
        ));
    }

    // If CPU 0 is already off, then return okay.
    if process_affinity_mask & 1 != 1 {
        return Ok(());
    }

    let clear_mask = !exclude;
    let new_mask = process_affinity_mask & clear_mask;
    if new_mask == 0 {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            format!("Only one CPU detected. Cannot change affinity. Last Error: {:X}", unsafe {
                GetLastError()
            }),
        ));
    }

    if !unsafe { SetProcessAffinityMask(*process_handle, new_mask) } {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            format!("Unable to set process affinity mask. Last Error: {:X}", unsafe {
                GetLastError()
            }),
        ));
    }

    Ok(())
}


#[test]
fn test_toml() {
    let toml = fs::read_to_string(CONFIG_PATH).unwrap();
    let hinstDLL = unsafe { GetModuleHandleA(0 as _) };
    let name = unsafe { get_file_name(hinstDLL) };
    let path = Path::new(&name);
    let working_dir = path.parent().unwrap().to_str().unwrap();
    fs::write(format!("{working_dir}/{CONFIG_PATH}"), toml).unwrap();

    let config = read_config_file(hinstDLL).expect("Could not read config");

    println!("{} {:?}", config.delay, config.exclude)
}
