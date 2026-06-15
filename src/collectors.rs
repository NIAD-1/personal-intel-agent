use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use rss::Channel;
use serde_json::Value;

use crate::{
    models::{IntelItem, SourceConfig},
    AppState,
};

pub async fn collect_intel(state: &AppState, query: Option<&str>) -> Vec<IntelItem> {
    let mut items = Vec::new();

    let rss_items = collect_rss(state).await;
    items.extend(rss_items);

    let hn_items = collect_hacker_news(state).await;
    items.extend(hn_items);

    let reddit_items = collect_reddit(
        state,
        query.unwrap_or("AI technology business finance travel relocation"),
    )
    .await;
    items.extend(reddit_items);

    if let Some(q) = query {
        items.extend(collect_tavily(state, q).await);
    } else {
        for interest in state.config.interests.iter().take(6) {
            items.extend(collect_tavily(state, interest).await);
        }
    }

    let mut items = dedupe(items);
    apply_preferences(
        &mut items,
        &state.config.interests,
        &state.config.blocked_keywords,
    );
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items.truncate(40);
    items
}

async fn collect_rss(state: &AppState) -> Vec<IntelItem> {
    let mut out = Vec::new();

    for source in default_sources() {
        match state.http.get(source.url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(body) => match Channel::read_from(&body[..]) {
                    Ok(channel) => {
                        for item in channel.items().iter().take(8) {
                            let title = item.title().unwrap_or("Untitled").trim();
                            let url = item.link().unwrap_or(source.url).trim();
                            if title.is_empty() || url.is_empty() {
                                continue;
                            }

                            out.push(IntelItem {
                                title: title.to_string(),
                                url: url.to_string(),
                                source: source.name.to_string(),
                                category: source.category.to_string(),
                                summary: clean_summary(item.description().unwrap_or_default()),
                                published_at: item.pub_date().and_then(parse_rss_date),
                                score: 10,
                            });
                        }
                    }
                    Err(error) => tracing::warn!("RSS parse failed for {}: {error}", source.name),
                },
                Err(error) => tracing::warn!("RSS body read failed for {}: {error}", source.name),
            },
            Err(error) => tracing::warn!("RSS fetch failed for {}: {error}", source.name),
        }
    }

    out
}

async fn collect_hacker_news(state: &AppState) -> Vec<IntelItem> {
    let url = "https://hn.algolia.com/api/v1/search_by_date?tags=story&hitsPerPage=12";
    let Ok(resp) = state.http.get(url).send().await else {
        return Vec::new();
    };
    let Ok(json) = resp.json::<Value>().await else {
        return Vec::new();
    };

    json["hits"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|hit| {
            let title = hit["title"].as_str()?.trim();
            let url = hit["url"]
                .as_str()
                .filter(|v| !v.trim().is_empty())
                .or_else(|| hit["story_url"].as_str())
                .unwrap_or("https://news.ycombinator.com");
            Some(IntelItem {
                title: title.to_string(),
                url: url.to_string(),
                source: "Hacker News".to_string(),
                category: "Technology".to_string(),
                summary: String::new(),
                published_at: hit["created_at"].as_str().and_then(parse_iso_date),
                score: 12,
            })
        })
        .collect()
}

async fn collect_reddit(state: &AppState, query: &str) -> Vec<IntelItem> {
    let q = urlencoding::encode(query);
    let url = format!("https://www.reddit.com/search.json?q={q}&sort=hot&t=day&limit=15");
    let Ok(resp) = state.http.get(url).send().await else {
        return Vec::new();
    };
    let Ok(json) = resp.json::<Value>().await else {
        return Vec::new();
    };

    json["data"]["children"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|child| {
            let data = &child["data"];
            let title = data["title"].as_str()?.trim();
            let permalink = data["permalink"].as_str().unwrap_or_default();
            let subreddit = data["subreddit_name_prefixed"]
                .as_str()
                .or_else(|| data["subreddit"].as_str())
                .unwrap_or("Reddit");
            let ups = data["ups"].as_i64().unwrap_or(0);
            Some(IntelItem {
                title: title.to_string(),
                url: format!("https://www.reddit.com{permalink}"),
                source: subreddit.to_string(),
                category: "Social Signals".to_string(),
                summary: clean_summary(data["selftext"].as_str().unwrap_or_default()),
                published_at: None,
                score: 8 + (ups.min(500) / 100) as i32,
            })
        })
        .collect()
}

async fn collect_tavily(state: &AppState, query: &str) -> Vec<IntelItem> {
    if state.config.tavily_api_key.trim().is_empty() {
        return Vec::new();
    }

    let payload = serde_json::json!({
        "query": query,
        "search_depth": "basic",
        "max_results": 8,
        "include_answer": false,
        "include_raw_content": false
    });

    let Ok(resp) = state
        .http
        .post("https://api.tavily.com/search")
        .header(
            "Authorization",
            format!("Bearer {}", state.config.tavily_api_key),
        )
        .json(&payload)
        .send()
        .await
    else {
        return Vec::new();
    };
    let Ok(json) = resp.json::<Value>().await else {
        return Vec::new();
    };

    json["results"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|result| {
            let title = result["title"].as_str()?.trim();
            let url = result["url"].as_str()?.trim();
            Some(IntelItem {
                title: title.to_string(),
                url: url.to_string(),
                source: result["source"].as_str().unwrap_or("Web").to_string(),
                category: infer_category(title),
                summary: clean_summary(result["content"].as_str().unwrap_or_default()),
                published_at: None,
                score: 11,
            })
        })
        .collect()
}

fn default_sources() -> Vec<SourceConfig> {
    vec![
        SourceConfig {
            name: "Reuters Business",
            category: "Business",
            url: "https://www.reutersagency.com/feed/?best-topics=business-finance&post_type=best",
        },
        SourceConfig {
            name: "Business Insider Africa",
            category: "Business Africa",
            url: "https://africa.businessinsider.com/rss",
        },
        SourceConfig {
            name: "TechCrunch",
            category: "Technology",
            url: "https://techcrunch.com/feed/",
        },
        SourceConfig {
            name: "The Verge",
            category: "Technology",
            url: "https://www.theverge.com/rss/index.xml",
        },
        SourceConfig {
            name: "MIT Technology Review",
            category: "AI",
            url: "https://www.technologyreview.com/feed/",
        },
        SourceConfig {
            name: "VentureBeat AI",
            category: "AI",
            url: "https://venturebeat.com/category/ai/feed/",
        },
        SourceConfig {
            name: "ESPN",
            category: "Sports",
            url: "https://www.espn.com/espn/rss/news",
        },
        SourceConfig {
            name: "Skift",
            category: "Travel",
            url: "https://skift.com/feed/",
        },
        SourceConfig {
            name: "Product Hunt",
            category: "Startups",
            url: "https://www.producthunt.com/feed",
        },
    ]
}

fn dedupe(items: Vec<IntelItem>) -> Vec<IntelItem> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();

    for item in items {
        let key = if !item.url.trim().is_empty() {
            normalize_url(&item.url)
        } else {
            item.title.to_ascii_lowercase()
        };
        if seen.insert(key) {
            out.push(item);
        }
    }

    out
}

fn apply_preferences(items: &mut Vec<IntelItem>, interests: &[String], blocked: &[String]) {
    items.retain(|item| {
        let haystack =
            format!("{} {} {}", item.title, item.summary, item.source).to_ascii_lowercase();
        !blocked
            .iter()
            .any(|keyword| haystack.contains(&keyword.to_ascii_lowercase()))
    });

    for item in items {
        let haystack =
            format!("{} {} {}", item.title, item.category, item.summary).to_ascii_lowercase();
        for interest in interests {
            if haystack.contains(&interest.to_ascii_lowercase()) {
                item.score += 5;
            }
        }
    }
}

fn clean_summary(value: &str) -> String {
    let without_tags = value
        .replace("<p>", "")
        .replace("</p>", " ")
        .replace("<br>", " ")
        .replace("<br/>", " ")
        .replace("&amp;", "&");
    without_tags
        .split_whitespace()
        .take(45)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_url(url: &str) -> String {
    url.split('?')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn parse_rss_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn parse_iso_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn infer_category(title: &str) -> String {
    let lower = title.to_ascii_lowercase();
    if lower.contains("ai") || lower.contains("openai") || lower.contains("anthropic") {
        "AI".to_string()
    } else if lower.contains("visa") || lower.contains("travel") || lower.contains("immigration") {
        "Travel/Relocation".to_string()
    } else if lower.contains("market") || lower.contains("stock") || lower.contains("inflation") {
        "Finance".to_string()
    } else {
        "Web".to_string()
    }
}
