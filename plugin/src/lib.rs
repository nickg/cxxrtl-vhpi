use std::ffi::{CString, CStr};
use std::ptr;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};
use std::io::{Read};
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use vhpi;
use bindings::{
    vhpiCbDataT, vhpiCbDataS, vhpi_register_cb,
    vhpiHandleT, vhpiTimeT, vhpiValueT,
    vhpiCbStartOfSimulation,
};

#[derive(Deserialize)]
struct GreetingRequest {
    #[serde(rename = "type")]
    msg_type: String,
    version: u32,
}

#[derive(Serialize)]
struct GreetingResponse<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    version: u32,
    commands: &'a [&'a str],
    events: &'a [&'a str],
    features: Features<'a>,
}

#[derive(Serialize)]
struct Features<'a> {
    item_values_encoding: &'a [&'a str],
}

#[derive(Deserialize)]
struct ListScopesRequest {
    #[serde(rename = "type")]
    msg_type: String,
    command: String,
    scope: Option<String>,
}

#[derive(Serialize)]
struct ListScopesResponse<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    command: &'a str,
    scopes: HashMap<&'a str, ScopeEntry<'a>>,
}

#[derive(Serialize)]
struct ScopeEntry<'a> {
    #[serde(rename = "type")]
    scope_type: &'a str, // e.g., "module"
    definition: ScopeDefinition<'a>,
    instantiation: ScopeInstantiation<'a>,
}

#[derive(Serialize)]
struct ScopeDefinition<'a> {
    src: Option<&'a str>,
    name: Option<&'a str>,
    attributes: HashMap<&'a str, ScopeAttribute<'a>>,
}

#[derive(Serialize)]
struct ScopeInstantiation<'a> {
    src: Option<&'a str>,
    attributes: HashMap<&'a str, ScopeAttribute<'a>>,
}

#[derive(Serialize)]
struct ScopeAttribute<'a> {
    #[serde(rename = "type")]
    attr_type: &'a str, // e.g., "unsigned_int"
    value: &'a str,
}

#[derive(Deserialize)]
struct GetSimulationStatusRequest {
    #[serde(rename = "type")]
    msg_type: String,
    command: String,
}

#[derive(Serialize)]
struct GetSimulationStatusResponse<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    command: &'a str,
    status: &'a str,
    latest_time: &'a str, // should be a fixed-precision string
}

#[derive(Serialize)]
struct ListItemsResponse<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    command: &'a str,
    items: HashMap<&'a str, ItemEntry<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ItemEntry<'a> {
    #[serde(rename = "node")]
    Node {
        src: &'a str,
        width: u32,
        lsb_at: u32,
        settable: bool,
        input: bool,
        output: bool,
        attributes: HashMap<&'a str, ItemAttribute<'a>>,
    },
    #[serde(rename = "memory")]
    Memory {
        src: Option<&'a str>,
        width: u32,
        lsb_at: u32,
        depth: u32,
        zero_at: u32,
        settable: bool,
        attributes: HashMap<&'a str, ItemAttribute<'a>>,
    },
}

#[derive(Serialize)]
struct ItemAttribute<'a> {
    #[serde(rename = "type")]
    attr_type: &'a str,
    value: serde_json::Value,
}

fn send_json<T: serde::Serialize>(stream: &mut TcpStream, value: &T) -> std::io::Result<()> {
    let mut payload = serde_json::to_vec(value)?;
    payload.push(0); // null terminator
    stream.write_all(&payload)
}

fn handle_greeting(req: &GreetingRequest, stream: &mut TcpStream) {
    println!("Received greeting (version {})", req.version);

    let response = GreetingResponse {
        msg_type: "greeting",
        version: 0,
        commands: &[
            "list_scopes",
            "list_items",
            "reference_items",
            "query_interval",
            "get_simulation_status",
            "run_simulation",
            "pause_simulation",
        ],
        events: &["simulation_paused", "simulation_finished"],
        features: Features {
            item_values_encoding: &["base64(u32)"],
        },
    };

    if let Err(e) = send_json(stream, &response) {
        eprintln!("Failed to send greeting response: {}", e);
    }
}

fn handle_get_simulation_status(_req: &ListScopesRequest, stream: &mut TcpStream) {
    let response = GetSimulationStatusResponse {
        msg_type: "response",
        command: "get_simulation_status",
        status: "paused",
        latest_time: "0.125000000000000",
    };

    if let Err(e) = send_json(stream, &response) {
        eprintln!("Failed to send get_simulation_status: {}", e);
    }
}

fn handle_list_items(_req: &ListScopesRequest, stream: &mut TcpStream) {
    let response = ListItemsResponse {
        msg_type: "response",
        command: "list_items",
        items: HashMap::new(),
    };

    if let Err(e) = send_json(stream, &response) {
        eprintln!("Failed to send get_simulation_status: {}", e);
    }
}

fn handle_list_scopes(_req: &ListScopesRequest, stream: &mut TcpStream) {
    println!("Handling list_scopes command");

    let mut scopes = HashMap::new();

    scopes.insert(
        "",
        ScopeEntry {
            scope_type: "module",
            definition: ScopeDefinition {
                src: None,
                name: None,
                attributes: HashMap::new(),
            },
            instantiation: ScopeInstantiation {
                src: None,
                attributes: HashMap::new(),
            },
        },
    );

    unsafe {
        let root: vhpiHandleT = bindings::vhpi_handle(bindings::vhpiOneToOneT_vhpiRootInst,
                                                      ptr::null_mut());

        let root_name_ptr = bindings::vhpi_get_str(bindings::vhpiStrPropertyT_vhpiNameP, root);
        let root_name = CStr::from_ptr(root_name_ptr as *const i8).to_str().unwrap();

        println!("Root name {}", root_name);

        scopes.insert(
            root_name,
            ScopeEntry {
                scope_type: "module",
                definition: ScopeDefinition {
                    src: None,
                    name: None,
                    attributes: HashMap::new(),
                },
                instantiation: ScopeInstantiation {
                    src: Some("top.py:50"),
                    attributes: HashMap::new(),
                },
            },
        );

        bindings::vhpi_release_handle(root);
    }

    let response = ListScopesResponse {
        msg_type: "response",
        command: "list_scopes",
        scopes,
    };

    if let Err(e) = send_json(stream, &response) {
        eprintln!("Failed to send list_scopes response: {}", e);
    }
}

fn handle_message(msg: &str, stream: &mut TcpStream) {
    if let Ok(req) = serde_json::from_str::<GreetingRequest>(msg) {
        if req.msg_type == "greeting" {
            handle_greeting(&req, stream);
            return;
        }
    }
    else if let Ok(req) = serde_json::from_str::<ListScopesRequest>(msg) {
        if req.msg_type == "command" && req.command == "list_scopes" {
            handle_list_scopes(&req, stream);
            return;
        }
        else if req.msg_type == "command" && req.command == "get_simulation_status" {
            handle_get_simulation_status(&req, stream);
            return;
        }
        else if req.msg_type == "command" && req.command == "list_items" {
            handle_list_items(&req, stream);
            return;
        }
    }

    eprintln!("Unhandled or invalid message: {}", msg);
}

fn read_null_terminated_message(buffer: &mut Vec<u8>) -> Option<String> {
    if let Some(null_pos) = buffer.iter().position(|&b| b == 0) {
        let message_bytes = buffer.drain(..null_pos).collect::<Vec<u8>>();
        buffer.remove(0); // remove the null byte
        String::from_utf8(message_bytes).ok()
    } else {
        None
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = Vec::new();
    let mut read_buf = [0u8; 1024];

    loop {
        match stream.read(&mut read_buf) {
            Ok(0) => {
                println!("Debugger disconnected");
                break;
            }
            Ok(n) => {
                buffer.extend_from_slice(&read_buf[..n]);

                while let Some(msg) = read_null_terminated_message(&mut buffer) {
                    handle_message(&msg, &mut stream);
                }
            }
            Err(e) => {
                eprintln!("Error reading from socket: {}", e);
                break;
            }
        }
    }
}

unsafe extern "C" fn start_of_sim(cb_data: *const vhpiCbDataS) {
    println!("Start of simulation callback triggered");

    const ADDR: &str = "127.0.0.1:4567";

    let listener = TcpListener::bind(ADDR).expect("Failed to bind socket");

    println!("CXXRTL plugin waiting for debugger on {}", ADDR);

    let (stream, addr) = listener.accept().expect("Failed to accept connection");

    println!("Debugger connected from {}", addr);

    handle_client(stream);
}

#[no_mangle]
pub extern "C" fn cxxrtl_startup() {
    vhpi::printf("CXXRTL plugin loaded");

    vhpi::Callback::new(vhpi::CbReason::StartOfSimulation, start_of_sim)
        .register();
}

type StartupFn = extern "C" fn();

#[no_mangle]
pub static vhpi_startup_routines: [Option<StartupFn>; 2] = [
    Some(cxxrtl_startup),
    None,
];
