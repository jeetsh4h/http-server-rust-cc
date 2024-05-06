use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

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

fn parse_path(http_response: &String) -> String {
    let whitespace_split_response: Vec<&str> = http_response.split_whitespace().collect();
    return whitespace_split_response[1].to_string();
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let mut incoming_buf_as_str = String::new();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");

                match _stream.read_to_string(&mut incoming_buf_as_str) {
                    Ok(size) => println!("Received {size} bytes"),
                    Err(err) => {
                        println!("error reading from stream: {err}");
                        continue;
                    }
                }
                let path = parse_path(&incoming_buf_as_str);
                if path == "/" {
                    respond_404(&mut _stream)
                } else {
                    respond_202(&mut _stream);
                }

            }
            Err(err) => {
                println!("error: {err}");
            }
        }
    }
}
