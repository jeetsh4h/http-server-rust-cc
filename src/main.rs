use httparse::{Request, EMPTY_HEADER};
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
                let mut headers = [EMPTY_HEADER; 4];
                let mut req = Request::new(&mut headers);
                match req.parse(&buf) {
                    Err(e) => {
                        println!("error parsing: {}", e);
                        break;
                    }
                    Ok(body_offset) => {
                        let method = req.method.unwrap();
                        let path = req.path.unwrap();

                        if method == "GET" {
                            match path {
                                "/" => respond_200(&mut stream).await,
                                _ if path.starts_with("/echo/") => {
                                    // no bounds check for 6th index
                                    respond_ok_body(&mut stream, "202", &path[6..], "text/plain").await
                                }
                                "/user-agent" => respond_user_agent(&mut stream, &req).await,
                                _ if path.starts_with("/files/") => {
                                    respond_file_get(&mut stream, &path).await
                                }
                                _ => respond_404(&mut stream).await,
                            }
                        } else if method == "POST" {
                            match path {
                                _ if path.starts_with("/files/") => {
                                    respond_file_put(&mut stream, &path, &buf, body_offset.unwrap(), &req).await
                                }
                                _ => respond_404_headers(&mut stream, "application/octet-stream").await,
                            }
                        } else {
                            respond_404(&mut stream).await;
                        }
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

async fn respond_404_headers(stream: &mut TcpStream, content_type: &str) {
    let status_line = "HTTP/1.1 404 Not Found\r\n";
    let content_type = format!("Content-Type: {}\r\n", content_type);
    let content_length = "Content-Length: 0\r\n";
    let body = "\r\n";

    let response = format!("{}{}{}{}", status_line, content_type, content_length, body);

    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_200(stream: &mut TcpStream) {
    match stream.write(b"HTTP/1.1 200 OK\r\n\r\n").await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_ok_body(stream: &mut TcpStream, response_code: &str, body: &str, content_type: &str) {
    let body_len = body.len();

    let status_line = format!("HTTP/1.1 {} OK\r\n", response_code);
    let content_type = format!("Content-Type: {}\r\n", content_type);
    let content_length = format!("Content-Length: {}\r\n", body_len);
    let body = format!("\r\n{}", body);

    let response = format!("{}{}{}{}", status_line, content_type, content_length, body);

    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    }
}

async fn respond_user_agent(stream: &mut TcpStream, req: &Request<'_, '_>) {
    let user_agent_header = req.headers.iter().find(|h| h.name == "User-Agent");
    match user_agent_header {
        Some(header) => unsafe {
            let body = std::str::from_utf8_unchecked(header.value);
            respond_ok_body(stream, "200", body, "text/plain").await;
        },
        None => println!("error: user-agent header not present"),
    }
}

async fn respond_file_get(stream: &mut TcpStream, file_path: &str) {
    let filename = &file_path[7..]; // no bounds check for 7th index

    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let dir = args.get(2).unwrap();     // ./your_server.sh --directory <directory>

    let full_path = Path::new(dir).join(filename);
    match File::open(full_path).await {
        Err(e) => {
            println!("error opening file: {}", e);
            respond_404_headers(stream, "application/octet-stream").await;
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
                    respond_ok_body(stream, "200", body, "application/octet-stream").await;
                },
            }
        }
    };
}

async fn respond_file_put(stream: &mut TcpStream, file_path: &str, buf: &[u8], body_offset: usize, req: &Request<'_, '_>) {
    let filename = &file_path[7..]; // no bounds check for 7th index

    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let dir = args.get(2).unwrap();     // ./your_server.sh --directory <directory>
    let full_path = Path::new(dir).join(filename);

    let body_len = req.headers.iter().find(|h| h.name == "Content-Length").unwrap().value[0];
    let body_len = usize::from(body_len);
    let body = &buf[body_offset..(body_offset + body_len)];

    println!("indices: {}.. {};", body_offset, body_offset + body_len);
    println!("buffer length: {}", body.len());

    match File::create(full_path).await {
        Err(e) => println!("error creating file: {}", e),
        Ok(mut file) => {
            match file.write_all(body).await {
                Err(e) => println!("error writing to file: {}", e),
                Ok(_) => respond_ok_body(stream, "201", "", "text/plain").await,
            }
        }
    }
}