use anyhow::Result;

/// short char time representation.
///
/// # Unit Test
///
/// ```
/// use sdp::util::*;
///
/// assert_eq!(short_time("1d").unwrap(), 86400.0);
/// assert_eq!(short_time("1h").unwrap(), 3600.0);
/// assert_eq!(short_time("1m").unwrap(), 60.0);
/// assert_eq!(short_time("1s").unwrap(), 1.0);
/// assert_eq!(short_time("100").unwrap(), 100.0);
/// ```
pub fn short_time(time: &str) -> Result<f64> {
    let (value, last) = time.split_at(time.len() - 1);
    Ok(match last {
        "d" => parse_f64(value)? * 86400.0,
        "h" => parse_f64(value)? * 3600.0,
        "m" => parse_f64(value)? * 60.0,
        "s" => parse_f64(value)?,
        _ => time.parse::<f64>()?
    })
}

/// placeholder char.
///
/// # Unit Test
///
/// ```
/// use sdp::util::*;
///
/// assert_eq!(placeholder("100"), Some("100"));
/// assert_eq!(placeholder("-"), None);
/// ```
pub fn placeholder(source: &str) -> Option<&str> {
    if source != "-" {
        Some(source)
    } else {
        None
    }
}

fn parse_f64(value: &str) -> Result<f64> {
    Ok(value.parse::<f64>()?)
}