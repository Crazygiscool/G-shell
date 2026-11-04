use crate::ipc::HubClient;

fn notify(client: &mut HubClient, title: &str, body: &str) -> i32 {
    let escaped_title = title.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_body = body.replace('\\', "\\\\").replace('"', "\\\"");
    let line = format!(
        "{{\"v\":1,\"type\":\"MSG_NOTIFY\",\"id\":\"gsh-notify\",\"from\":\"gsh\",\"title\":\"{}\",\"body\":\"{}\"}}",
        escaped_title, escaped_body
    );
    if let Err(e) = client.send_line(&line) {
        eprintln!("hub: {}", e);
        return 1;
    }
    0
}

pub fn ipc_builtin(args: &[&str]) -> i32 {
    if args.first().copied() == Some("path") {
        println!("{}", crate::ipc::hub_socket_path());
        return 0;
    }

    let mut client = match HubClient::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("hub: {}", e);
            return 1;
        }
    };

    let ack = match client.handshake() {
        Ok(line) => line,
        Err(e) => {
            eprintln!("hub: handshake: {}", e);
            return 1;
        }
    };
    println!("{}", ack);

    match args.first().copied() {
        None | Some("connect") => if let Ok(line) = client.read_line() { println!("{}", line) },
        Some("ping") => {
            if let Err(e) = client.send_line(
                r#"{"v":1,"type":"MSG_PING","id":"gsh-ping","target":"gsh","from":"gsh"}"#,
            ) {
                eprintln!("hub: {}", e);
                return 1;
            }
            if let Ok(line) = client.read_line() { println!("{}", line) }
        }
        Some("query") => {
            if let Err(e) = client.send_line(
                r#"{"v":1,"type":"MSG_OBJECT_QUERY","id":"gsh-query","from":"gsh"}"#,
            ) {
                eprintln!("hub: {}", e);
                return 1;
            }
            if let Ok(line) = client.read_line() { println!("{}", line) }
        }
        Some("notify") => {
            let title = args.get(1).copied().unwrap_or("G-Shell");
            let body = args.iter().skip(2).cloned().collect::<Vec<_>>().join(" ");
            return notify(&mut client, title, &body);
        }
        Some("send") => {
            let payload = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
            if payload.is_empty() {
                eprintln!("hub: usage: hub send JSON_LINE");
                return 2;
            }
            if let Err(e) = client.send_line(&payload) {
                eprintln!("hub: {}", e);
                return 1;
            }
            if let Ok(line) = client.read_line() { println!("{}", line) }
        }
        Some(other) => {
            eprintln!("hub: unknown subcommand: {}", other);
            return 2;
        }
    }

    0
}