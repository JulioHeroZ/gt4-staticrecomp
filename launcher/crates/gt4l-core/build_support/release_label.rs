/// Formats a UI/build label without changing the numeric version used by the legacy updater and receipts.
pub fn display_version(numeric_version: &str, label: &str) -> Result<String, &'static str> {
    if label.is_empty() {
        return Ok(numeric_version.to_string());
    }
    let digits = label.strip_prefix("rc").unwrap_or("");
    if digits.is_empty() || digits.starts_with('0') || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("GT4_RELEASE_LABEL must be empty or rc<N> (positive N, no leading zero)");
    }
    Ok(format!("{numeric_version}-{label}"))
}
