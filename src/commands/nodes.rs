pub fn nodes(args: &[&str]) -> i32 {
    match args.first().copied() {
        None | Some("list") | Some("query") => crate::commands::ipc::ipc_builtin(&["query"]),
        Some("ping") => crate::commands::ipc::ipc_builtin(&["ping"]),
        Some("notify") => {
            if args.len() < 2 {
                eprintln!("nodes: usage: nodes notify TITLE [BODY...]");
                return 2;
            }
            let mut forwarded = vec!["notify"];
            forwarded.extend(args.iter().skip(1).copied());
            crate::commands::ipc::ipc_builtin(&forwarded)
        }
        Some(other) => {
            eprintln!("nodes: unknown subcommand: {}", other);
            2
        }
    }
}