pub enum HistoryAction {
    Load(String),
    Write(String),
    Append(String), // New variant for appending
    None,
}

pub fn history(entries: &[String], args: &[String]) -> HistoryAction {
    // Detect flags: -r (load), -w (write), and -a (append)
    if let Some(pos) = args.iter().position(|arg| arg == "-r") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Load(path.clone());
        }
    }
    
    if let Some(pos) = args.iter().position(|arg| arg == "-w") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Write(path.clone());
        }
    }

    if let Some(pos) = args.iter().position(|arg| arg == "-a") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Append(path.clone());
        }
    }

    // Normal history listing logic
    let n = args.first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(entries.len());

    let start_index = entries.len().saturating_sub(n);
    let display_subset = &entries[start_index..];
    let width = entries.len().to_string().len();

    for (i, entry) in display_subset.iter().enumerate() {
        println!("  {:>width$}  {}", i + start_index + 1, entry, width = width);
    }

    HistoryAction::None
}
