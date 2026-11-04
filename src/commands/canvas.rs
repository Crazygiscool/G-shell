pub fn canvas(args: &[&str]) -> i32 {
    match args.first().copied() {
        None | Some("list") | Some("query") => crate::commands::ipc::ipc_builtin(&["query"]),
        Some("ping") => crate::commands::ipc::ipc_builtin(&["ping"]),
        Some("connect") => crate::commands::ipc::ipc_builtin(&["connect"]),
        Some("notify") => {
            if args.len() < 2 {
                eprintln!("canvas: usage: canvas notify TITLE [BODY...]");
                return 2;
            }
            let mut forwarded = vec!["notify"];
            forwarded.extend(args.iter().skip(1).copied());
            crate::commands::ipc::ipc_builtin(&forwarded)
        }
        Some("layout") => {
            let payload = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
            if payload.is_empty() {
                eprintln!("canvas: usage: canvas layout JSON");
                return 2;
            }
            crate::commands::ipc::ipc_builtin(&["send", &payload])
        }
        Some(other) => {
            eprintln!("canvas: unknown subcommand: {}", other);
            2
        }
    }
}