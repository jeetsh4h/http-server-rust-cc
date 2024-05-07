use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use httparse::{Request, EMPTY_HEADER};
use std::error::Error;

fn respond_202(stream: &mut TcpStream) {
    match stream.write(b"HTTP/1.1 200 OK\r\n\r\n") {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

fn respond_404(stream: &mut TcpStream) {
    match stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n") {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

fn respond_202_body(stream: &mut TcpStream, body: &str) {
    let body_len = body.len();

    let status_line = "HTTP/1.1 200 OK\r\n";
    let content_type = "Content-Type: text/plain\r\n";
    let content_length = format!("Content-Length: {}\r\n", body_len);
    let body = format!("\r\n{}", body);

    let response = format!("{}{}{}{}", status_line, content_type, content_length, body);

    match stream.write(response.as_bytes()) {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    }
}

fn respond_echo(stream: &mut TcpStream, echo_path: &str) {
    let path_parsed: Vec<&str> = echo_path.split('/').collect();
    assert_eq!(path_parsed[1], "echo");
    assert!(path_parsed.len() >= 3);

    let echo_string = path_parsed[2];
    respond_202_body(stream, echo_string);
}

fn parse_request_path(buf: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut headers = [ EMPTY_HEADER; 4 ];
    let mut req = Request::new(&mut headers);
    match req.parse(buf) {
        Err(e) => Err(Box::new(e)),
        Ok(_body_offset) => {
            let path = req.path.unwrap();
            Ok(path.to_string())
        }
    }
}

fn respond_user_agent(stream: &mut TcpStream, buf: &[u8]) {
    let mut headers = [ EMPTY_HEADER; 4 ];
    let mut req = Request::new(&mut headers);

    match req.parse(buf) {
        Ok(_body_offset) => {
            let user_agent_header = req.headers.iter().find(|h| h.name == "User-Agent");
            match user_agent_header {
                Some(header) => {
                    unsafe {
                        let body = std::str::from_utf8_unchecked(header.value);
                        respond_202_body(stream, body);
                    }
                },
                None => println!("error: user-agent header not present"),
            }
        }
        Err(e) => println!("error parsing headers: {}", e),
    }
}

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let mut buf = [0; 1024];

        match stream {
            Ok(mut _stream) => {
                // recv function,,, buffer size is one byte
                match _stream.read(&mut buf) {
                    Ok(size) => {
                        println!("Received {} bytes", size);

                        // parse path from request and call proper function
                        match parse_request_path(&buf) {
                            Ok(path) => match path.as_str() {
                                "/" => respond_202(&mut _stream),
                                r"/echo/*" => respond_echo(&mut _stream, &path),
                                "/user-agent" => respond_user_agent(&mut _stream, &buf),
                                _ => respond_404(&mut _stream),
                            },
                            Err(e) => println!("error parsing: {}", e),
                        }
                    }
                    Err(e) => println!("error receiving: {}", e),
                }
            }
            Err(e) => println!("error connecting: {}", e),
        }
    }
}
