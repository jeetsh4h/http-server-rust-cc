use flate2::{write::GzEncoder, Compression};
use httparse::{Header, Request, EMPTY_HEADER};
use std::{io::Write, path::Path};
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
        // IMPORTANT
        // the below loop assumes that all the requests currently being recieved
        // can be stored withing one byte.
        // Another assumption that is being made is that, everything in the request
        // is encoded using utf-8 or ascii-us
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
                            // match case to endpoints
                            match path {
                                "/" => respond_no_headers(&mut stream, "200", "OK").await,
                                _ if path.starts_with("/echo/") => {
                                    // no bounds check for 6th index
                                    respond_echo(&mut stream, &req, &path[6..]).await
                                }
                                "/user-agent" => respond_user_agent(&mut stream, &req).await,
                                _ if path.starts_with("/files/") => {
                                    respond_file_get(&mut stream, &path).await
                                }
                                _ => respond_no_headers(&mut stream, "404", "Not Found").await,
                            }
                        } else if method == "POST" {
                            // match case to endpoints
                            match path {
                                _ if path.starts_with("/files/") => {
                                    respond_file_put(
                                        &mut stream,
                                        &path,
                                        &buf,
                                        body_offset.unwrap(),
                                        &req,
                                    )
                                    .await
                                }
                                _ => {
                                    let headers_404 = vec![
                                        Header {
                                            name: "Content-Type",
                                            value: "text/plain".as_bytes(),
                                        },
                                        Header {
                                            name: "Content-Length",
                                            value: "0".as_bytes(),
                                        },
                                    ];
                                    respond_headers(
                                        &mut stream,
                                        "404",
                                        "Not Found",
                                        &headers_404,
                                        "",
                                    )
                                    .await
                                }
                            }
                        } else {
                            respond_no_headers(&mut stream, "404", "Not Found").await;
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

async fn respond_headers(
    stream: &mut TcpStream,
    response_code: &str,
    response_reason: &str,
    headers: &Vec<Header<'_>>,
    body: &str,
) {
    let status_line = format!("HTTP/1.1 {} {}\r\n", response_code, response_reason);
    let headers_str: String = headers
        .iter()
        .map(|h| format!("{}: {}\r\n", h.name, String::from_utf8_lossy(h.value)))
        .collect::<Vec<String>>()
        .join("");
    let body_fmt = format!("\r\n{}", body);

    let response = format!("{}{}{}", status_line, headers_str, body_fmt);
    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_no_headers(stream: &mut TcpStream, response_code: &str, response_reason: &str) {
    let status_line = format!("HTTP/1.1 {} {}\r\n", response_code, response_reason);
    let response = format!("{}\r\n", status_line);

    match stream.write(response.as_bytes()).await {
        Ok(size) => println!("Sent {size} bytes"),
        Err(err) => println!("error writing to stream: {err}"),
    };
}

async fn respond_user_agent(stream: &mut TcpStream, req: &Request<'_, '_>) {
    let user_agent_header = req
        .headers
        .iter()
        .find(|h| h.name.to_lowercase() == "user-agent");
    match user_agent_header {
        Some(header) => unsafe {
            let body = std::str::from_utf8_unchecked(header.value);
            let body_len = body.len().to_string();
            let headers = vec![
                Header {
                    name: "Content-Type",
                    value: "text/plain".as_bytes(),
                },
                Header {
                    name: "Content-Length",
                    value: body_len.as_bytes(),
                },
            ];
            respond_headers(stream, "200", "OK", &headers, body).await;
        },
        None => println!("error: user-agent header not present"),
    }
}

async fn respond_file_get(stream: &mut TcpStream, file_path: &str) {
    let filename = &file_path[7..]; // no bounds check for 7th index

    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let dir = args.get(2).unwrap(); // ./your_server.sh --directory <directory>

    let full_path = Path::new(dir).join(filename);
    match File::open(full_path).await {
        Err(e) => {
            println!("error opening file: {}", e);
            let headers_404 = vec![
                Header {
                    name: "Content-Type",
                    value: "application/octet".as_bytes(),
                },
                Header {
                    name: "Content-Length",
                    value: "0".as_bytes(),
                },
            ];
            respond_headers(stream, "404", "Not Found", &headers_404, "").await;
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
                    let body_len = buf.len().to_string();
                    let headers = vec![
                        Header {
                            name: "Content-Type",
                            value: "application/octet".as_bytes(),
                        },
                        Header {
                            name: "Content-Length",
                            value: body_len.as_bytes(),
                        },
                    ];
                    respond_headers(stream, "404", "Not Found", &headers, body).await
                },
            }
        }
    };
}

async fn respond_file_put(
    stream: &mut TcpStream,
    file_path: &str,
    buf: &[u8],
    body_offset: usize,
    req: &Request<'_, '_>,
) {
    let filename = &file_path[7..]; // no bounds check for 7th index

    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let dir = args.get(2).unwrap(); // ./your_server.sh --directory <directory>
    let full_path = Path::new(dir).join(filename);

    let body_len_buf = req
        .headers
        .iter()
        .find(|h| h.name.to_lowercase() == "content-length")
        .unwrap()
        .value;
    let body_len: usize = std::str::from_utf8(body_len_buf).unwrap().parse().unwrap();

    let body = &buf[body_offset..(body_offset + body_len)];

    match File::create(full_path).await {
        Err(e) => println!("error creating file: {}", e),
        Ok(mut file) => match file.write_all(body).await {
            Err(e) => println!("error writing to file: {}", e),
            Ok(_) => {
                let headers = vec![
                    Header {
                        name: "Content-Type",
                        value: "application/octet".as_bytes(),
                    },
                    Header {
                        name: "Content-Length",
                        value: "0".as_bytes(),
                    },
                ];
                respond_headers(stream, "201", "Created", &headers, "").await;
            }
        },
    }
}

async fn respond_echo(stream: &mut TcpStream, req: &Request<'_, '_>, echo_str: &str) {
    let decoded_body_len = echo_str.len().to_string();
    let encoding_header = req
        .headers
        .iter()
        .find(|h| h.name.to_lowercase() == "accept-encoding");

    match encoding_header {
        Some(header) => {
            match std::str::from_utf8(header.value)
                .unwrap()
                .split(",")
                .map(|s| s.trim())
                .find(|s| *s == "gzip")
            {
                Some(_) => {
                    let encoded_bytes = compress_data(echo_str);
                    let encoded_body_len = encoded_bytes.len().to_string();

                    let encoded_bytes_as_string =
                        String::from_utf8_lossy(&encoded_bytes[..encoded_bytes.len()]).to_string();

                    let headers = vec![
                        Header {
                            name: "Content-Encoding",
                            value: "gzip".as_bytes(),
                        },
                        Header {
                            name: "Content-Type",
                            value: "text/plain".as_bytes(),
                        },
                        Header {
                            name: "Content-Length",
                            value: encoded_body_len.as_bytes(),
                        },
                    ];

                    respond_headers(
                        stream,
                        "200",
                        "OK",
                        &headers,
                        encoded_bytes_as_string.as_str(),
                    )
                    .await;
                }
                None => {
                    let headers = vec![
                        Header {
                            name: "Content-Type",
                            value: "text/plain".as_bytes(),
                        },
                        Header {
                            name: "Content-Length",
                            value: decoded_body_len.as_bytes(),
                        },
                    ];

                    respond_headers(stream, "200", "OK", &headers, echo_str).await;
                }
            }
        }
        None => {
            let headers = vec![
                Header {
                    name: "Content-Type",
                    value: "text/plain".as_bytes(),
                },
                Header {
                    name: "Content-Length",
                    value: decoded_body_len.as_bytes(),
                },
            ];

            respond_headers(stream, "200", "OK", &headers, echo_str).await;
        }
    }
}

fn compress_data(data: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).unwrap();
    return encoder.finish().unwrap();
}
