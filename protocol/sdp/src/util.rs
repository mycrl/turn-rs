use anyhow::{
    Result,
    anyhow
};

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

/// # Unit Test
///
/// ```
/// use sdp::util::*;
///
/// assert_eq!(tuple2_from_split("1/2", '/', "").unwrap(), ("1", "2"));
/// assert_eq!(tuple2_from_split("1:2", ':', "").unwrap(), ("1", "2"));
/// assert!(tuple2_from_split("1 2", ':', "").is_err());
/// ```
pub fn tuple2_from_split<'a>(
    value: &'a str, 
    pat: char,
    msg: &'static str
) -> Result<(&'a str, &'a str)> {
    let mut split = value.split(pat);
    let v1 = split.next().ok_or_else(|| anyhow!(msg))?;
    let v2 = split.next().ok_or_else(|| anyhow!(msg))?;
    if split.next().is_some() {
        return Err(anyhow!(msg))
    }
    
    Ok((v1, v2))
}

/// # Unit Test
///
/// ```
/// use sdp::util::*;
///
/// assert_eq!(tuple3_from_split("1/2/3", '/', "").unwrap(), ("1", "2", "3"));
/// assert_eq!(tuple3_from_split("1:2:3", ':', "").unwrap(), ("1", "2", "3"));
/// assert!(tuple3_from_split("1 2 3", ':', "").is_err());
/// ```
pub fn tuple3_from_split<'a>(
    value: &'a str, 
    pat: char, 
    msg: &'static str
) -> Result<(&'a str, &'a str, &'a str)> {
    let mut split = value.split(pat);
    let v1 = split.next().ok_or_else(|| anyhow!(msg))?;
    let v2 = split.next().ok_or_else(|| anyhow!(msg))?;
    let v3 = split.next().ok_or_else(|| anyhow!(msg))?;
    if split.next().is_some() {
        return Err(anyhow!(msg))
    }
    
    Ok((v1, v2, v3))
}

fn parse_f64(value: &str) -> Result<f64> {
    Ok(value.parse::<f64>()?)
}