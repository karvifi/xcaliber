// SPDX-License-Identifier: AGPL-3.0-only
//! Minimal loopback-only OpenAI-compatible chat client.

use serde_json::{json, Value};
use std::env;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:8000";
const DEFAULT_MODEL: &str = "kimi-k3-local";
const DEFAULT_KEY: &str = "xcaliber-local";
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
    host: String,
    port: u16,
    path: String,
}

#[derive(Debug)]
struct Options {
    endpoint: Endpoint,
    model: String,
    api_key: String,
    prompt: String,
    max_tokens: u16,
    json: bool,
}

fn endpoint(input: &str) -> Result<Endpoint, String> {
    let value = input
        .strip_prefix("http://")
        .ok_or("--api-url must use loopback HTTP, for example http://127.0.0.1:8000")?;
    if value.contains('@') || value.contains('#') || value.contains('?') {
        return Err(
            "--api-url must not contain credentials, fragments, or query parameters".into(),
        );
    }
    let (authority, base_path) = value.split_once('/').unwrap_or((value, ""));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .map_err(|_| "--api-url contains an invalid port")?,
        ),
        None => (authority, 80),
    };
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err("--api-url is restricted to 127.0.0.1 or localhost".into());
    }
    let base = base_path.trim_matches('/');
    let path = if base.is_empty() {
        "/v1/chat/completions".to_string()
    } else if base.ends_with("v1/chat/completions") {
        format!("/{base}")
    } else if base.ends_with("v1") {
        format!("/{base}/chat/completions")
    } else {
        return Err("--api-url path must be empty, /v1, or /v1/chat/completions".into());
    };
    Ok(Endpoint {
        host: host.to_string(),
        port,
        path,
    })
}

fn option_text(args: &[OsString], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} needs a value"))
}

fn parse(args: &[OsString]) -> Result<Options, String> {
    let mut url = env::var("XCALIBER_API_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    let mut model = env::var("XCALIBER_API_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let mut api_key =
        env::var("XCALIBER_LOCAL_API_KEY").unwrap_or_else(|_| DEFAULT_KEY.to_string());
    let mut prompt = None;
    let mut max_tokens = 128u16;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].to_string_lossy().as_ref() {
            "--api-url" => url = option_text(args, &mut index, "--api-url")?,
            "--model" => model = option_text(args, &mut index, "--model")?,
            "--api-key" => api_key = option_text(args, &mut index, "--api-key")?,
            "--prompt" => prompt = Some(option_text(args, &mut index, "--prompt")?),
            "--max-tokens" => {
                max_tokens = option_text(args, &mut index, "--max-tokens")?
                    .parse::<u16>()
                    .map_err(|_| "--max-tokens must be an integer from 1 to 4096")?;
                if !(1..=4096).contains(&max_tokens) {
                    return Err("--max-tokens must be from 1 to 4096".into());
                }
            }
            "--json" => json = true,
            other => return Err(format!("unknown option for chat: {other}")),
        }
        index += 1;
    }
    if api_key
        .chars()
        .any(|character| matches!(character, '\r' | '\n'))
    {
        return Err("--api-key must not contain a newline".into());
    }
    Ok(Options {
        endpoint: endpoint(&url)?,
        model,
        api_key,
        prompt: prompt.ok_or("chat needs --prompt TEXT")?,
        max_tokens,
        json,
    })
}

fn decode_chunked(mut input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    loop {
        let line_end = input
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("invalid chunked HTTP response")?;
        let size_text = std::str::from_utf8(&input[..line_end])
            .map_err(|_| "invalid chunk size")?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16).map_err(|_| "invalid chunk size")?;
        input = &input[line_end + 2..];
        if size == 0 {
            return Ok(output);
        }
        if input.len() < size + 2 || &input[size..size + 2] != b"\r\n" {
            return Err("truncated chunked HTTP response".into());
        }
        output.extend_from_slice(&input[..size]);
        if output.len() > MAX_RESPONSE_BYTES {
            return Err("local API response exceeded 64 MiB".into());
        }
        input = &input[size + 2..];
    }
}

fn response_body(response: &[u8]) -> Result<&[u8], String> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("local API returned an invalid HTTP response")?;
    let headers = std::str::from_utf8(&response[..split])
        .map_err(|_| "local API returned invalid HTTP headers")?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 2") && !status.starts_with("HTTP/1.0 2") {
        let body = String::from_utf8_lossy(&response[split + 4..]);
        return Err(format!("local API returned {status}: {}", body.trim()));
    }
    Ok(&response[split + 4..])
}

fn request(options: &Options) -> Result<Value, String> {
    let body = serde_json::to_vec(&json!({
        "model": options.model,
        "messages": [{"role": "user", "content": options.prompt}],
        "stream": false,
        "max_tokens": options.max_tokens
    }))
    .map_err(|error| format!("cannot encode local API request: {error}"))?;
    let address = (options.endpoint.host.as_str(), options.endpoint.port)
        .to_socket_addrs()
        .map_err(|error| format!("cannot resolve loopback endpoint: {error}"))?
        .next()
        .ok_or("loopback endpoint did not resolve")?;
    if !address.ip().is_loopback() {
        return Err("resolved API address is not loopback".into());
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|error| format!("cannot connect to local API: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(600)))
        .map_err(|error| format!("cannot set local API timeout: {error}"))?;
    let headers = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        options.endpoint.path,
        options.endpoint.host,
        options.endpoint.port,
        options.api_key,
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|error| format!("cannot write local API request: {error}"))?;
    let mut response = Vec::new();
    stream
        .take((MAX_RESPONSE_BYTES + 64 * 1024) as u64)
        .read_to_end(&mut response)
        .map_err(|error| format!("cannot read local API response: {error}"))?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("local API returned an invalid HTTP response")?;
    let chunked = String::from_utf8_lossy(&response[..split])
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    let raw = response_body(&response)?;
    let decoded = if chunked {
        decode_chunked(raw)?
    } else {
        raw.to_vec()
    };
    serde_json::from_slice(&decoded)
        .map_err(|error| format!("local API returned invalid JSON: {error}"))
}

pub fn run(args: &[OsString]) -> Result<i32, String> {
    let options = parse(args)?;
    let response = request(&options)?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&response)
                .map_err(|error| format!("cannot encode chat response: {error}"))?
        );
    } else {
        let content = response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or("local API response has no choices[0].message.content")?;
        println!("{content}");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::{decode_chunked, endpoint, request, Endpoint, Options};
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use std::thread;

    #[test]
    fn only_loopback_http_is_accepted() {
        assert_eq!(
            endpoint("http://127.0.0.1:8000/v1").unwrap(),
            Endpoint {
                host: "127.0.0.1".into(),
                port: 8000,
                path: "/v1/chat/completions".into()
            }
        );
        assert!(endpoint("https://127.0.0.1:8000").is_err());
        assert!(endpoint("http://example.com:8000").is_err());
    }

    #[test]
    fn chunked_json_body_is_decoded() {
        assert_eq!(
            decode_chunked(b"3\r\n{\"a\r\n4\r\n\":1}\r\n0\r\n\r\n").unwrap(),
            b"{\"a\":1}"
        );
    }

    #[test]
    fn loopback_request_round_trip() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_bytes = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                assert!(
                    read > 0,
                    "client closed before sending the complete request"
                );
                request_bytes.extend_from_slice(&buffer[..read]);
                let Some(header_end) = request_bytes
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .and_then(|value| value.parse::<usize>().ok())
                    })
                    .expect("request should contain Content-Length");
                if request_bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let request_text = String::from_utf8_lossy(&request_bytes);
            assert!(request_text.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
            assert!(request_text.contains("Authorization: Bearer test-key\r\n"));
            let body = br#"{"choices":[{"message":{"content":"local test passed"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
        });
        let options = Options {
            endpoint: Endpoint {
                host: "127.0.0.1".into(),
                port,
                path: "/v1/chat/completions".into(),
            },
            model: "local-test".into(),
            api_key: "test-key".into(),
            prompt: "hello".into(),
            max_tokens: 8,
            json: false,
        };
        let response = request(&options).unwrap();
        server.join().unwrap();
        assert_eq!(
            response
                .pointer("/choices/0/message/content")
                .and_then(serde_json::Value::as_str),
            Some("local test passed")
        );
    }
}
