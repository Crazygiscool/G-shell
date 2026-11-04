use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub fn hub_socket_path() -> String {
    std::env::var("ARCOS_HUB_SOCKET")
        .or_else(|_| std::env::var("GEORGE_HUB_SOCKET"))
        .unwrap_or_else(|_| "/run/system-manager/ipc.sock".to_string())
}

pub struct HubClient {
    stream: UnixStream,
}

impl HubClient {
pub fn connect() -> std::io::Result<Self> {
    Self::connect_at(hub_socket_path())
}

pub fn connect_at(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    Ok(HubClient { stream })
}

    pub fn handshake(&mut self) -> std::io::Result<String> {
        writeln!(
            self.stream,
            "{{\"v\":1,\"type\":\"gsh_ready\",\"id\":\"gsh-hs\",\"name\":\"gsh\",\"capabilities\":[\"shell\"]}}"
        )?;
        self.stream.flush()?;
        self.read_line()
    }

    pub fn send_line(&mut self, line: &str) -> std::io::Result<()> {
        writeln!(self.stream, "{}", line)?;
        self.stream.flush()
    }

    pub fn read_line(&mut self) -> std::io::Result<String> {
        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        Ok(line.trim_end_matches('\n').to_string())
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_socket_path_default() {
        unsafe {
            std::env::remove_var("ARCOS_HUB_SOCKET");
            std::env::remove_var("GEORGE_HUB_SOCKET");
        }
        assert_eq!(hub_socket_path(), "/run/system-manager/ipc.sock");
    }

    #[test]
    fn test_hub_client_roundtrip() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;
        use std::thread;
        let path = "/tmp/gsh_ipc_roundtrip.sock";
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).unwrap();
        let path_buf = path.to_string();
        let server = thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut out = s.try_clone().unwrap();
            writeln!(out, r#"{{"v":1,"type":"handshake_ack","id":"gsh-hs","status":"ok"}}"#).unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            writeln!(out, "pong-test").unwrap();
            out.flush().unwrap();
        });

        let mut client = HubClient::connect_at(path_buf).unwrap();
        let ack = client.handshake().unwrap();
        assert!(ack.contains("handshake_ack"));
        client.send_line("payload").unwrap();
        assert_eq!(client.read_line().unwrap(), "pong-test");
        server.join().unwrap();
        let _ = std::fs::remove_file(path);

    #[test]
    fn test_hub_builtin_query_via_mock() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;
        use std::thread;

        let path = "/tmp/gsh_hub_builtin_query.sock";
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).unwrap();
        let server_path = path.to_string();
        let server = thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut out = s.try_clone().unwrap();
            writeln!(out, r#"{{"v":1,"type":"handshake_ack","id":"gsh-hs","status":"ok"}}"#).unwrap();
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("MSG_OBJECT_QUERY"));
            writeln!(out, r#"{{"v":1,"type":"MSG_UI_RENDER_STATE","objects":[]}}"#).unwrap();
            out.flush().unwrap();
        });

        unsafe {
            std::env::set_var("ARCOS_HUB_SOCKET", &server_path);
        }
        let code = crate::commands::ipc::ipc_builtin(&["query"]);
        unsafe {
            std::env::remove_var("ARCOS_HUB_SOCKET");
        }
        let _ = std::fs::remove_file(&server_path);
        server.join().unwrap();
        assert_eq!(code, 0);
    }

    }
}
