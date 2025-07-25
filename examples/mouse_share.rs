use rdev::{Event, EventType, SimulateError, display_size, grab, simulate};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn send_loop(mut stream: TcpStream) {
    let (width, _) = display_size().expect("Could not get screen size");
    let mut remote = false;
    grab(move |event| {
        if remote {
            if let EventType::MouseMove { x, y } = event.event_type {
                if x <= 0.0 {
                    remote = false;
                    let _ = simulate(&EventType::MouseMove { x: 1.0, y });
                }
            }
            if serde_json::to_writer(&mut stream, &event).is_ok() {
                let _ = stream.write_all(b"\n");
            }
            None
        } else {
            if let EventType::MouseMove { x, y } = event.event_type {
                if x >= (width - 1) as f64 {
                    remote = true;
                    let _ = simulate(&EventType::MouseMove {
                        x: (width - 1) as f64,
                        y,
                    });
                    if serde_json::to_writer(&mut stream, &event).is_ok() {
                        let _ = stream.write_all(b"\n");
                    }
                    return None;
                }
            }
            Some(event)
        }
    })
    .expect("grab failed");
}

fn recv_loop(listener: TcpListener) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                                if let Err(SimulateError) = simulate(&event.event_type) {
                                    eprintln!("Failed to simulate event {:?}", event);
                                }
                                thread::sleep(Duration::from_millis(5));
                            }
                        }
                    }
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <send <host:port>|recv <port>>", args[0]);
        return;
    }
    match args[1].as_str() {
        "send" => {
            let addr = &args[2];
            let stream = TcpStream::connect(addr).expect("Cannot connect to server");
            send_loop(stream);
        }
        "recv" => {
            let addr = format!("0.0.0.0:{}", args[2]);
            let listener = TcpListener::bind(addr).expect("Cannot bind to port");
            recv_loop(listener);
        }
        _ => eprintln!("Invalid mode"),
    }
}
