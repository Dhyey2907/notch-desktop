//! Weather for the Home dashboard: your position from Windows location services (rounded to
//! about a kilometre, kept in memory only, never logged), the forecast from Open-Meteo and
//! the place name from BigDataCloud's client geocoder — neither needs an account or key.

use serde_json::Value;

use super::{Weather, WeatherStatus};

/// WMO weather code -> (text, icon key) as Open-Meteo reports it.
pub fn describe(code: i64, is_day: bool) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear", if is_day { "sun" } else { "moon" }),
        1 => ("Mostly clear", if is_day { "sun" } else { "moon" }),
        2 => ("Partly cloudy", if is_day { "cloud-sun" } else { "cloud-moon" }),
        3 => ("Cloudy", "cloud"),
        45 | 48 => ("Fog", "fog"),
        51 | 53 | 55 | 56 | 57 => ("Drizzle", "drizzle"),
        61 | 63 | 66 => ("Rain", "rain"),
        65 | 67 => ("Heavy rain", "rain"),
        71 | 73 | 75 | 77 => ("Snow", "snow"),
        80..=82 => ("Showers", "rain"),
        85 | 86 => ("Snow showers", "snow"),
        95..=99 => ("Thunderstorm", "storm"),
        _ => ("—", "cloud"),
    }
}

/// Round a coordinate to two decimals (~1 km): enough for weather, and less precise to send.
pub fn coarse(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn parse_forecast(json: &Value) -> Option<(f64, i64, bool)> {
    let current = json.get("current")?;
    let temp = current.get("temperature_2m")?.as_f64()?;
    let code = current.get("weather_code")?.as_i64()?;
    let is_day = current.get("is_day").and_then(Value::as_i64).unwrap_or(1) == 1;
    temp.is_finite().then_some((temp, code, is_day))
}

pub fn parse_place(json: &Value) -> Option<String> {
    ["city", "locality", "principalSubdivision", "countryName"]
        .iter()
        .filter_map(|k| json.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(|s| s.chars().take(40).collect())
}

#[derive(Debug)]
pub enum Located {
    At(f64, f64),
    /// Location is off, or desktop apps aren't allowed to use it.
    NoAccess,
    Failed,
}

/// Ask Windows where this PC is (blocks; call off the async runtime).
#[cfg(windows)]
pub fn locate() -> Located {
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator, PositionAccuracy};
    let access = Geolocator::RequestAccessAsync().and_then(|op| op.get());
    match access {
        Ok(GeolocationAccessStatus::Allowed) => {}
        Ok(_) => return Located::NoAccess,
        Err(_) => return Located::Failed,
    }
    let position = Geolocator::new()
        .and_then(|g| {
            g.SetDesiredAccuracy(PositionAccuracy::Default)?;
            g.GetGeopositionAsync()?.get()
        })
        .and_then(|p| p.Coordinate()?.Point()?.Position());
    match position {
        Ok(p) => Located::At(coarse(p.Latitude), coarse(p.Longitude)),
        // Access was allowed but location services can't place us right now.
        Err(_) => Located::NoAccess,
    }
}

#[cfg(not(windows))]
pub fn locate() -> Located {
    Located::Failed
}

pub async fn fetch(http: &reqwest::Client, lat: f64, lon: f64) -> Option<Weather> {
    let forecast: Value = http
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", lat.to_string()),
            ("longitude", lon.to_string()),
            ("current", "temperature_2m,weather_code,is_day".into()),
            ("timezone", "auto".into()),
        ])
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let (temp, code, is_day) = parse_forecast(&forecast)?;
    let place = async {
        let json: Value = http
            .get("https://api.bigdatacloud.net/data/reverse-geocode-client")
            .query(&[
                ("latitude", lat.to_string()),
                ("longitude", lon.to_string()),
                ("localityLanguage", "en".into()),
            ])
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        parse_place(&json)
    }
    .await;
    let (condition, icon) = describe(code, is_day);
    Some(Weather {
        status: WeatherStatus::Ready,
        temperature: Some(temp.round() as i32),
        condition: Some(condition.into()),
        icon: Some(icon.into()),
        place,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn describes_weather_codes() {
        assert_eq!(describe(2, true), ("Partly cloudy", "cloud-sun"));
        assert_eq!(describe(0, false), ("Clear", "moon"));
        assert_eq!(describe(81, true).1, "rain");
        assert_eq!(describe(96, true).0, "Thunderstorm");
    }

    #[test]
    fn rounds_position_to_about_a_kilometre() {
        assert_eq!(coarse(23.022_505), 23.02);
        assert_eq!(coarse(72.571_365), 72.57);
    }

    #[test]
    fn parses_forecast_and_place() {
        let f = json!({ "current": { "temperature_2m": 18.4, "weather_code": 2, "is_day": 1 } });
        assert_eq!(parse_forecast(&f), Some((18.4, 2, true)));
        assert_eq!(parse_forecast(&json!({ "error": true })), None);
        assert_eq!(
            parse_place(&json!({ "city": "", "locality": "Ahmedabad" })).as_deref(),
            Some("Ahmedabad")
        );
        assert_eq!(parse_place(&json!({})), None);
    }
}
