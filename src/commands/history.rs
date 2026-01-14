pub enum HistoryAction {
    Load(String),
    None,
}

pub fn history(entries: &[String], args: &[String]) -> HistoryAction {
    // 1. Detect the -r flag (e.g., history -r my_file.txt)
    // We check if "-r" exists and if there is a filename following it
    if let Some(pos) = args.iter().position(|arg| arg == "-r") {
        if let Some(path) = args.get(pos + 1) {
            return HistoryAction::Load(path.clone());
        }
    }

    // 2. Normal history listing logic
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