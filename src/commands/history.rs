pub enum HistoryAction {
    Load(String),
    Write(String),
    Append(String),
    None,
}

pub fn history(entries: &[String], args: &[String]) -> HistoryAction {
    // 1. Detect flags: -r (load), -w (write), and -a (append)
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

    // 2. Normal history listing logic
    let n = args.first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(entries.len());

    let start_index = entries.len().saturating_sub(n);
    let display_subset = &entries[start_index..];
    
    // FIX FOR CODECRAFTERS: Use fixed width of 5 for the index.
    // Standard bash uses a 5-character wide column for indices up to 99999.
    // This results in the exact spacing required: "    8  echo apple mango"
    for (i, entry) in display_subset.iter().enumerate() {
        let display_num = i + start_index + 1;
        println!("{:>5}  {}", display_num, entry);
    }

    HistoryAction::None
}
