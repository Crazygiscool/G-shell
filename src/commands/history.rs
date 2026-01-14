pub fn history(history_entries: &[String]) {
    for (i, entry) in history_entries.iter().enumerate() {
        // Formats with a right-aligned index (typical shell style)
        println!("  {:>3}  {}", i + 1, entry);
    }
}
