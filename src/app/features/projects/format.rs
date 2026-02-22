/// Format estimated minutes for display (e.g. "—", "45 min", "2 h", "1 h 30 min").
pub fn format_estimated_minutes(minutes: i64) -> String {
    if minutes == 0 {
        "—".to_string()
    } else if minutes < 60 {
        format!("{} min", minutes)
    } else {
        let h = minutes / 60;
        let m = minutes % 60;
        if m == 0 {
            format!("{} h", h)
        } else {
            format!("{} h {} min", h, m)
        }
    }
}
