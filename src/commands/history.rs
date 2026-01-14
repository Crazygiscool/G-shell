pub fn history(entries: &[String], args: &[String]) {
    // 1. Determine the limit (n). Default to the full history length.
    let n = args.first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(entries.len());

    // 2. Calculate the slice window (last n items)
    let start_index = entries.len().saturating_sub(n);
    let display_subset = &entries[start_index..];

    // 3. Dynamic padding based on the total number of entries
    let width = entries.len().to_string().len();

    for (i, entry) in display_subset.iter().enumerate() {
        // Calculate the absolute history index for display
        println!("  {:>width$}  {}", i + start_index + 1, entry, width = width);
    }
}
