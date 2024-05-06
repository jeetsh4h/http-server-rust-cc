use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

// fn respond_202(stream: &mut TcpStream) {
//     match stream.write(b"HTTP/1.1 200 OK\r\n\r\n") {
//         Ok(size) => println!("Sent {size} bytes"),
//         Err(err) => println!("error writing to stream: {err}"),
//     };
// }

// fn respond_404(stream: &mut TcpStream) {
//     match stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n") {
//         Ok(size) => println!("Sent {size} bytes"),
//         Err(err) => println!("error writing to stream: {err}"),
//     };
// }

fn parse_path(http_response: &String) -> String {
    let whitespace_split_response: Vec<&str> = http_response.split_whitespace().collect();

    // this assumes that I will always receive a valid HTTP request
    // I am only returning a path
    return whitespace_split_response[1].to_string();
}

fn echo_respond(stream: &mut TcpStream, echo_string: &String ) {
    let status_line = "HTTP/1.1 200 OK\r\n";
    let content_type = "Content-Type: text/plain\r\n";
    let content_length = format!("Content-Length: {}\r\n\r\n", echo_string.len());
    let echo_fmt = format!("{}", echo_string);

    let mut response = String::new();
    response.push_str(status_line);
    response.push_str(content_type);
    response.push_str(&content_length);
    response.push_str(&echo_fmt);

    match stream.write(response.as_bytes()) {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let mut buffer = [0; 1024];

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                
                // recv1024 function
                match _stream.read(&mut buffer) {
                    Ok(size) => println!("Received {size} bytes"),
                    Err(err) => {
                        println!("error reading from stream: {err}");
                        continue;
                    }
                }
                
                // parse the buffer, turn into the string, then give a response accordingly 
                let buf_as_str = unsafe { std::str::from_utf8_unchecked(&buffer) };
                let path = parse_path(&buf_as_str.to_string());
                // if path == "/" {
                //     respond_202(&mut _stream);
                // } else {
                //     respond_404(&mut _stream);
                // }
                
                // further parse the path that results after parsing the buffer,
                // give response accordingly
                let path_parsed: Vec<&str> = path.split('/').collect();
                if path_parsed[1].to_string() == "echo" {
                    echo_respond(&mut _stream, &path_parsed[2].to_string());
                } else {
                    println!("not an echo command");
                }

            }
            Err(err) => {
                println!("error: {err}");
            }
        }
    }
}
