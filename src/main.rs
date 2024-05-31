use flate2::write::GzEncoder;
use flate2::Compression;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn gzip_encode(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn handle_connection(mut stream: TcpStream, directory: &str) {
    let mut buf_reader = BufReader::new(&mut stream);
    let mut http_request = String::new();
    let mut content_length = 0;
    let mut accept_encoding = String::new();

    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if line.starts_with("Content-Length: ") {
            content_length = line[16..].trim().parse().unwrap();
        } else if line.starts_with("Accept-Encoding: ") {
            accept_encoding = line[17..].trim().to_lowercase();
        }
        http_request.push_str(&line);
    }

    let response = match http_request.lines().next() {
        Some("GET / HTTP/1.1") => {
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_string()
        }
        Some(line) if line.starts_with("GET /echo/") && line.ends_with(" HTTP/1.1") => {
            let text = &line[10..line.len() - 9];
            let mut response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                text.len(),
                text
            );

            if accept_encoding.contains("gzip") {
                let compressed_data = gzip_encode(text.as_bytes());
                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                    compressed_data.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.write_all(&compressed_data).unwrap();
                return;
            }

            response
        }
        Some(line) if line.starts_with("GET /user-agent") && line.ends_with(" HTTP/1.1") => {
            if let Some(user_agent_line) = http_request
                .lines()
                .find(|line| line.starts_with("User-Agent: "))
            {
                let user_agent = &user_agent_line["User-Agent: ".len()..];
                let content_length = user_agent.len();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    content_length, user_agent
                )
            } else {
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n"
                    .to_string()
            }
        }
        Some(line) if line.starts_with("GET /files/") && line.ends_with(" HTTP/1.1") => {
            let filename = &line[11..line.len() - 9];
            let file_path = format!("{}/{}", directory, filename);
            match fs::read_to_string(file_path) {
                Ok(file_content) => {
                    let content_length = file_content.len();
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                        content_length, file_content
                    )
                }
                Err(_) => {
                    "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_string()
                }
            }
        }
        Some(line) if line.starts_with("POST /files/") && line.ends_with(" HTTP/1.1") => {
            let filename = &line[12..line.len() - 9];
            let file_path = format!("{}/{}", directory, filename);
            let mut body = vec![0; content_length];
            buf_reader.read_exact(&mut body).unwrap();

            match File::create(file_path) {
                Ok(mut file) => {
                    file.write_all(&body).unwrap();
                    "HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_string()
                }
                Err(_) => {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_string()
                }
            }
        }
        _ => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n"
            .to_string(),
    };

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main() {
    println!("Logs from your program will appear aqui!");

    let directory = "/file/";

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let directory = directory.to_string();
                thread::spawn(move || {
                    handle_connection(stream, &directory);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
