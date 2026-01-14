pub enum HistoryAction {
    Load(String),
    Write(String),
    Append(String),
    None,
}

pub fn history(entries: &[String], args: &[String]) -> HistoryAction {
    // 1. Detect flags: -r (load), -w (write), and -a (append)
    // We check for the flag and ensure there is an argument following it to use as a path.
    if let Some(pos) = args.iter().position(|arg| arg == "-r") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Load(path.clone());
        } else {
            eprintln!("history: -r: option requires an argument");
            return HistoryAction::None;
        }
    }
    
    if let Some(pos) = args.iter().position(|arg| arg == "-w") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Write(path.clone());
        } else {
            eprintln!("history: -w: option requires an argument");
            return HistoryAction::None;
        }
    }

    if let Some(pos) = args.iter().position(|arg| arg == "-a") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Append(path.clone());
        } else {
            eprintln!("history: -a: option requires an argument");
            return HistoryAction::None;
        }
    }

    // 2. Normal history listing logic (e.g., "history" or "history 5")
    // Parse the first argument if it's a number; otherwise, show the full list.
    let n = args.first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(entries.len());

    // Calculate the window of entries to display
    let start_index = entries.len().saturating_sub(n);
    let display_subset = &entries[start_index..];
    
    // Calculate dynamic padding based on the total history size
    let width = entries.len().to_string().len();

    for (i, entry) in display_subset.iter().enumerate() {
        // Use the absolute index (position in full history) for the display number
        println!(
            "  {:>width$}  {}", 
            i + start_index + 1, 
            entry, 
            width = width
        );
    }

    HistoryAction::None
}
