use super::{Segment, SegmentData};
use crate::config::{InputData, RateLimits, SegmentId};
use crate::utils::credentials;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

#[derive(Debug, Deserialize)]
struct ApiUsageResponse {
    five_hour: Option<UsagePeriod>,
    seven_day: Option<UsagePeriod>,
}

#[derive(Debug, Deserialize)]
struct UsagePeriod {
    utilization: f64,
    resets_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiUsageCache {
    #[serde(default)]
    scope: String,
    five_hour_utilization: Option<f64>,
    seven_day_utilization: Option<f64>,
    #[serde(default)]
    five_hour_resets_at: Option<String>,
    #[serde(default, alias = "resets_at")]
    seven_day_resets_at: Option<String>,
    cached_at: String,
}

#[derive(Default)]
pub struct UsageSegment;

impl UsageSegment {
    pub fn new() -> Self {
        Self
    }

    fn normalize_percentage(value: Option<f64>) -> Option<f64> {
        value
            .filter(|percentage| percentage.is_finite())
            .map(|percentage| percentage.clamp(0.0, 100.0))
    }

    fn format_percentage(value: Option<f64>) -> String {
        match Self::normalize_percentage(value) {
            Some(percentage) if percentage.fract() == 0.0 => format!("{:.0}%", percentage),
            Some(percentage) => format!("{:.1}%", percentage),
            None => "-".to_string(),
        }
    }

    fn get_circle_icon(utilization: f64) -> String {
        match utilization.clamp(0.0, 100.0).round() as u8 {
            0..=12 => "\u{f0a9e}".to_string(),
            13..=25 => "\u{f0a9f}".to_string(),
            26..=37 => "\u{f0aa0}".to_string(),
            38..=50 => "\u{f0aa1}".to_string(),
            51..=62 => "\u{f0aa2}".to_string(),
            63..=75 => "\u{f0aa3}".to_string(),
            76..=87 => "\u{f0aa4}".to_string(),
            _ => "\u{f0aa5}".to_string(),
        }
    }

    fn render_usage(
        five_hour: Option<f64>,
        seven_day: Option<f64>,
        source: &str,
    ) -> Option<SegmentData> {
        let five_hour = Self::normalize_percentage(five_hour);
        let seven_day = Self::normalize_percentage(seven_day);
        if five_hour.is_none() && seven_day.is_none() {
            return None;
        }

        let icon_percentage = seven_day.or(five_hour).unwrap_or(0.0);
        let primary = format!("5h {}", Self::format_percentage(five_hour));
        let secondary = format!("· 7d {}", Self::format_percentage(seven_day));

        let mut metadata = HashMap::new();
        metadata.insert(
            "dynamic_icon".to_string(),
            Self::get_circle_icon(icon_percentage),
        );
        metadata.insert(
            "five_hour_utilization".to_string(),
            five_hour.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        metadata.insert(
            "seven_day_utilization".to_string(),
            seven_day.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        metadata.insert("source".to_string(), source.to_string());

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    }

    fn render_native(rate_limits: &RateLimits) -> Option<SegmentData> {
        let five_hour = rate_limits
            .five_hour
            .as_ref()
            .and_then(|window| window.used_percentage);
        let seven_day = rate_limits
            .seven_day
            .as_ref()
            .and_then(|window| window.used_percentage);
        Self::render_usage(five_hour, seven_day, "native")
    }

    fn get_cache_path() -> Option<std::path::PathBuf> {
        Some(crate::config::paths::api_usage_cache_file())
    }

    fn load_cache(&self) -> Option<ApiUsageCache> {
        let content = std::fs::read_to_string(Self::get_cache_path()?).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_cache(&self, cache: &ApiUsageCache) {
        let Some(cache_path) = Self::get_cache_path() else {
            return;
        };
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(cache) {
            let _ = std::fs::write(cache_path, json);
        }
    }

    fn cache_scope(api_base_url: &str, token: &str) -> String {
        let mut hasher = DefaultHasher::new();
        api_base_url.hash(&mut hasher);
        token.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn is_cache_valid(cache: &ApiUsageCache, scope: &str, cache_duration: u64) -> bool {
        if cache.scope != scope {
            return false;
        }
        let Ok(cached_at) = DateTime::parse_from_rfc3339(&cache.cached_at) else {
            return false;
        };
        let elapsed = Utc::now().signed_duration_since(cached_at.with_timezone(&Utc));
        elapsed.num_seconds() >= 0 && elapsed.num_seconds() < cache_duration as i64
    }

    fn render_scoped_cache(
        cache: &ApiUsageCache,
        scope: &str,
        source: &str,
    ) -> Option<SegmentData> {
        if cache.scope != scope {
            return None;
        }
        Self::render_usage(
            cache.five_hour_utilization,
            cache.seven_day_utilization,
            source,
        )
    }

    fn get_proxy_from_settings() -> Option<String> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let content =
            std::fs::read_to_string(std::path::Path::new(&home).join(".claude/settings.json"))
                .ok()?;
        let settings: serde_json::Value = serde_json::from_str(&content).ok()?;
        settings
            .get("env")?
            .get("HTTPS_PROXY")
            .or_else(|| settings.get("env")?.get("HTTP_PROXY"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    }

    fn fetch_api_usage(
        api_base_url: &str,
        token: &str,
        timeout_secs: u64,
        version: Option<&str>,
    ) -> Option<ApiUsageResponse> {
        let url = format!("{}/api/oauth/usage", api_base_url.trim_end_matches('/'));
        let user_agent = version
            .map(|value| format!("claude-code/{}", value))
            .unwrap_or_else(|| "claude-code".to_string());
        let agent = if let Some(proxy_url) = Self::get_proxy_from_settings() {
            ureq::Proxy::new(&proxy_url)
                .map(|proxy| {
                    ureq::Agent::config_builder()
                        .proxy(Some(proxy))
                        .build()
                        .new_agent()
                })
                .unwrap_or_else(|_| ureq::Agent::new_with_defaults())
        } else {
            ureq::Agent::new_with_defaults()
        };

        agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", &user_agent)
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(
                timeout_secs.clamp(1, 10),
            )))
            .build()
            .call()
            .ok()?
            .into_body()
            .read_json()
            .ok()
    }

    fn collect_api_fallback(&self, input: &InputData) -> Option<SegmentData> {
        let config = crate::config::Config::load().ok()?;
        let segment_config = config
            .segments
            .iter()
            .find(|segment| segment.id == SegmentId::Usage);
        let api_base_url = segment_config
            .and_then(|segment| segment.options.get("api_base_url"))
            .and_then(|value| value.as_str())
            .unwrap_or("https://api.anthropic.com");
        let cache_duration = segment_config
            .and_then(|segment| segment.options.get("cache_duration"))
            .and_then(|value| value.as_u64())
            .unwrap_or(300);
        let timeout = segment_config
            .and_then(|segment| segment.options.get("timeout"))
            .and_then(|value| value.as_u64())
            .unwrap_or(2);

        let token = credentials::get_oauth_token()?;
        let scope = Self::cache_scope(api_base_url, &token);
        let cached_data = self.load_cache();
        if let Some(cache) = cached_data
            .as_ref()
            .filter(|cache| Self::is_cache_valid(cache, &scope, cache_duration))
        {
            return Self::render_scoped_cache(cache, &scope, "api-cache");
        }

        let Some(response) =
            Self::fetch_api_usage(api_base_url, &token, timeout, input.version.as_deref())
        else {
            return cached_data
                .as_ref()
                .and_then(|cache| Self::render_scoped_cache(cache, &scope, "api-cache-stale"));
        };
        let five_hour = response.five_hour.as_ref().map(|period| period.utilization);
        let seven_day = response.seven_day.as_ref().map(|period| period.utilization);
        let five_hour_reset = response
            .five_hour
            .as_ref()
            .and_then(|period| period.resets_at.clone());
        let seven_day_reset = response
            .seven_day
            .as_ref()
            .and_then(|period| period.resets_at.clone());
        self.save_cache(&ApiUsageCache {
            scope,
            five_hour_utilization: five_hour,
            seven_day_utilization: seven_day,
            five_hour_resets_at: five_hour_reset,
            seven_day_resets_at: seven_day_reset.clone(),
            cached_at: Utc::now().to_rfc3339(),
        });
        Self::render_usage(five_hour, seven_day, "api")
    }
}

impl Segment for UsageSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        if let Some(rate_limits) = input.rate_limits.as_ref() {
            if let Some(data) = Self::render_native(rate_limits) {
                return Some(data);
            }
        }
        self.collect_api_fallback(input)
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiUsageCache, UsageSegment};
    use crate::config::{RateLimitWindow, RateLimits};

    #[test]
    fn native_usage_labels_both_windows() {
        let limits = RateLimits {
            five_hour: Some(RateLimitWindow {
                used_percentage: Some(8.0),
                resets_at: None,
            }),
            seven_day: Some(RateLimitWindow {
                used_percentage: Some(2.0),
                resets_at: None,
            }),
        };
        let data = UsageSegment::render_native(&limits).unwrap();

        assert_eq!(data.primary, "5h 8%");
        assert_eq!(data.secondary, "· 7d 2%");
        assert_eq!(data.metadata.get("source").unwrap(), "native");
    }

    #[test]
    fn usage_percentages_are_clamped() {
        assert_eq!(UsageSegment::format_percentage(Some(123.0)), "100%");
        assert_eq!(UsageSegment::format_percentage(Some(-4.0)), "0%");
        assert_eq!(UsageSegment::format_percentage(None), "-");
    }

    #[test]
    fn stale_cache_fallback_requires_the_same_scope() {
        let cache = ApiUsageCache {
            scope: "account-a".to_string(),
            five_hour_utilization: Some(12.0),
            seven_day_utilization: Some(34.0),
            five_hour_resets_at: None,
            seven_day_resets_at: None,
            cached_at: "2000-01-01T00:00:00Z".to_string(),
        };

        let stale =
            UsageSegment::render_scoped_cache(&cache, "account-a", "api-cache-stale").unwrap();

        assert_eq!(stale.primary, "5h 12%");
        assert_eq!(stale.secondary, "· 7d 34%");
        assert_eq!(stale.metadata.get("source").unwrap(), "api-cache-stale");
        assert!(
            UsageSegment::render_scoped_cache(&cache, "account-b", "api-cache-stale").is_none()
        );
    }
}
