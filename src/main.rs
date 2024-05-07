use tokio::{
    task,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use httparse::{Request, EMPTY_HEADER};
use std::error::Error;
use regex::Regex;

async fn handle_connection(mut stream: TcpStream) {
    let mut buf = vec![0; 1024];

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,     // disconnect connections
            Ok(size) => {
                println!("Received {} bytes", size);
                
                // parse path from request and call proper function
                match parse_request_path(&buf).await {
                    Ok(path) => {
                        let echo_regex = Regex::new(r"^/echo/[\x00-\x7F]*$").unwrap();
                        match path.as_str() {
                            "/" => respond_202(&mut stream).await,
                            _ if echo_regex.is_match(&path) => respond_echo(&mut stream, &path).await,
                            "/user-agent" => respond_user_agent(&mut stream, &buf).await,
                            _ => respond_404(&mut stream).await,
                        }
                    },
                    Err(e) => {
                        println!("error parsing: {}", e);
                        break;
                    }
                }
            }    
            Err(e) => {
                println!("error receiving: {}", e);
                break;
            } 
        }
    }
}

async fn respond_404(stream: &mut TcpStream) {
    match stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_202(stream: &mut TcpStream) {
    match stream.write(b"HTTP/1.1 200 OK\r\n\r\n").await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_202_body(stream: &mut TcpStream, body: &str) {
    let body_len = body.len();

    let status_line = "HTTP/1.1 200 OK\r\n";
    let content_type = "Content-Type: text/plain\r\n";
    let content_length = format!("Content-Length: {}\r\n", body_len);
    let body = format!("\r\n{}", body);

    let response = format!("{}{}{}{}", status_line, content_type, content_length, body);

    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    }
}

async fn respond_echo(stream: &mut TcpStream, echo_path: &str) {
    let path_parsed: Vec<&str> = echo_path.split('/').collect();
    assert_eq!(path_parsed[1], "echo");
    assert!(path_parsed.len() >= 3);

    let echo_string = path_parsed[2];
    respond_202_body(stream, echo_string).await;
}

async fn parse_request_path(buf: &[u8]) -> Result<String, Box<dyn Error>> {
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

async fn respond_user_agent(stream: &mut TcpStream, buf: &[u8]) {
    let mut headers = [ EMPTY_HEADER; 4 ];
    let mut req = Request::new(&mut headers);

    match req.parse(buf) {
        Ok(_body_offset) => {
            let user_agent_header = req.headers.iter().find(|h| h.name == "User-Agent");
            match user_agent_header {
                Some(header) => {
                    unsafe {
                        let body = std::str::from_utf8_unchecked(header.value);
                        respond_202_body(stream, body).await;
                    }
                },
                None => println!("error: user-agent header not present"),
            }
        }
        Err(e) => println!("error parsing headers: {}", e),
    }
}


#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    println!("Server listening on port 4221");

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        task::spawn(async move {
            let _ = handle_connection(stream);
        });
    }
}
