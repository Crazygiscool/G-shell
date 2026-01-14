pub enum HistoryAction {
    Load(String),
    Write(String),
    Append(String),
    Clear,
    None,
}

pub fn history(entries: &[String], args: &[String], default_path: &str) -> HistoryAction {
    // 1. Detect Clear Flag
    if args.iter().any(|arg| arg == "-c") {
        return HistoryAction::Clear;
    }

    // 2. Detect -r (read/load)
    if let Some(pos) = args.iter().position(|arg| arg == "-r") {
        let path = args.get(pos + 1).cloned().unwrap_or_else(|| default_path.to_string());
        return HistoryAction::Load(path);
    }
    
    // 3. Detect -w (write/overwrite)
    if let Some(pos) = args.iter().position(|arg| arg == "-w") {
        let path = args.get(pos + 1).cloned().unwrap_or_else(|| default_path.to_string());
        return HistoryAction::Write(path);
    }

    // 4. Detect -a (append)
    if let Some(pos) = args.iter().position(|arg| arg == "-a") {
        let path = args.get(pos + 1).cloned().unwrap_or_else(|| default_path.to_string());
        return HistoryAction::Append(path);
    }

    // 5. Normal history listing logic
    let n = args.first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(entries.len());

    let start_index = entries.len().saturating_sub(n);
    let display_subset = &entries[start_index..];
    
    // Formatting for CodeCrafters: 5-char width for index
    for (i, entry) in display_subset.iter().enumerate() {
        let display_num = i + start_index + 1;
        println!("{:>5}  {}", display_num, entry);
    }

    HistoryAction::None
}