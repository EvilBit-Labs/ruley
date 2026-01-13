use indicatif::{ProgressBar, ProgressStyle};

pub fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    // Use unwrap_or_else to fall back to default style if template parsing fails
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("#>-");
    pb.set_style(style);
    pb
}
