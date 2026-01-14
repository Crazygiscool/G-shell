pub fn history(entries: &[String]) {
    // 1. Calculate the number of digits in the last index to set padding
    // e.g., if total is 1050, width is 4. If total is 50, width is 2.
    let width = entries.len().to_string().len();

    for (i, entry) in entries.iter().enumerate() {
        // 2. Use dynamic alignment width
        // The '*' in '{:>width$}' tells Rust to use the 'width' variable for padding
        println!("  {:>width$}  {}", i + 1, entry, width = width);
    }
}
