use crate::error::{Result, bad};

pub(crate) fn text(s: &str, label: &str, max: usize) -> Result<String> {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > max || s.chars().any(char::is_control) {
        return Err(bad(&format!(
            "{label}: Bitte 1 bis {max} Zeichen eingeben."
        )));
    }
    Ok(s.into())
}

pub(crate) fn email(s: &str) -> Result<String> {
    let s = text(s, "E-Mail", 254)?.to_lowercase();
    if !s.contains('@') || s.starts_with('@') || s.ends_with('@') || s.contains(char::is_whitespace)
    {
        return Err(bad("Bitte eine gültige E-Mail-Adresse eingeben."));
    }
    Ok(s)
}

pub(crate) fn valid_date(date: &str) -> bool {
    let b = date.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let year = date[..4].parse::<u32>().unwrap();
    let month = date[5..7].parse::<usize>().unwrap();
    let day = date[8..].parse::<u32>().unwrap();
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && (1..=12).contains(&month) && day > 0 && day <= days[month - 1]
}
