use std::ffi::CString;
use std::ptr;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};

use bindings::{
    vhpiCbDataT, vhpiCbDataS, vhpi_register_cb,
    vhpiHandleT, vhpiTimeT, vhpiValueT,
    vhpiCbStartOfSimulation,
};

unsafe extern "C" fn start_of_sim(cb_data: *const vhpiCbDataS) {
    println!("Start of simulation callback triggered");
    // do any startup logic here
}

#[no_mangle]
pub extern "C" fn cxxrtl_startup() {
    let msg = CString::new("CXXRTL plugin loaded").unwrap();
    unsafe {
        bindings::vhpi_printf(msg.as_ptr());
    }

    const ADDR: &str = "127.0.0.1:4567"; // or use a configurable port

    // Bind to the port and listen
    let listener = TcpListener::bind(ADDR).expect("Failed to bind socket");

    println!("CXXRTL plugin waiting for debugger on {}", ADDR);

    // Accept a connection (blocking)
    let (stream, addr) = listener.accept().expect("Failed to accept connection");

    println!("Debugger connected from {}", addr);


    let mut cb_data = vhpiCbDataS {
        reason: vhpiCbStartOfSimulation as i32,
        cb_rtn: Some(start_of_sim),
        obj: ptr::null_mut(),
        time: ptr::null_mut(),
        value: ptr::null_mut(),
        user_data: ptr::null_mut(),
    };

    unsafe {
        let _handle: vhpiHandleT = vhpi_register_cb(&mut cb_data as *mut _, 0);
    }
}

type StartupFn = extern "C" fn();

#[no_mangle]
pub static vhpi_startup_routines: [Option<StartupFn>; 2] = [
    Some(cxxrtl_startup),
    None,
];
