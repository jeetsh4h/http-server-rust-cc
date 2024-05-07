use httparse::{Error, Request, EMPTY_HEADER};
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task,
};

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    println!("Server listening on port 4221");

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        task::spawn(async move {
            handle_connection(stream).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let mut buf = vec![0; 1024];

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break, // disconnect connections
            Ok(size) => {
                println!("Received {} bytes", size);

                // parse path from request and call proper function
                match parse_request_path(&buf).await {
                    Ok(path) => {
                        match path.as_str() {
                            "/" => respond_202(&mut stream).await,
                            _ if path.starts_with("/echo/") => {
                                respond_202_body(&mut stream, &path[6..], "text/plain").await
                            } // no bounds check for 6th index
                            "/user-agent" => respond_user_agent(&mut stream, &buf).await,
                            _ if path.starts_with("/files/") => {
                                respond_file(&mut stream, &path).await
                            }
                            _ => respond_404(&mut stream).await,
                        }
                    }
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

async fn parse_request_path(buf: &[u8]) -> Result<String, Error> {
    let mut headers = [EMPTY_HEADER; 4];
    let mut req = Request::new(&mut headers);
    match req.parse(buf) {
        Err(e) => Err(e),
        Ok(_body_offset) => {
            let path = req.path.unwrap();
            Ok(path.to_string())
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

async fn respond_202_body(stream: &mut TcpStream, body: &str, content_type: &str) {
    let body_len = body.len();

    let status_line = "HTTP/1.1 200 OK\r\n";
    let content_type = format!("Content-Type: {}\r\n", content_type);
    let content_length = format!("Content-Length: {}\r\n", body_len);
    let body = format!("\r\n{}", body);

    let response = format!("{}{}{}{}", status_line, content_type, content_length, body);

    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    }
}

async fn respond_user_agent(stream: &mut TcpStream, buf: &[u8]) {
    let mut headers = [EMPTY_HEADER; 4];
    let mut req = Request::new(&mut headers);

    match req.parse(buf) {
        Ok(_body_offset) => {
            let user_agent_header = req.headers.iter().find(|h| h.name == "User-Agent");
            match user_agent_header {
                Some(header) => unsafe {
                    let body = std::str::from_utf8_unchecked(header.value);
                    respond_202_body(stream, body, "text/plain").await;
                },
                None => println!("error: user-agent header not present"),
            }
        }
        Err(e) => println!("error parsing headers: {}", e),
    }
}

async fn respond_file(stream: &mut TcpStream, file_path: &str) {
    let filename = &file_path[7..]; // no bounds check for 7th index

    let args: Vec<String> = std::env::args().collect(); //        0              1          2
    let dir = args.get(2).unwrap();     // ./your_server.sh --directory <directory>

    let full_path = Path::new(dir).join(filename);
    match File::open(full_path).await {
        Err(e) => {
            println!("error opening file: {}", e);
            respond_404(stream).await;
        }
        Ok(file) => {
            let mut file = file;
            let mut buf = vec![];
            match file.read_to_end(&mut buf).await {
                Err(e) => {
                    println!("error reading file: {}", e);
                }
                Ok(_) => unsafe {
                    let body = std::str::from_utf8_unchecked(&buf);
                    respond_202_body(stream, body, "application/octet-stream").await;
                },
            }
        }
    };
}
