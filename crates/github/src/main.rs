use std::{collections::BTreeMap, env, fs, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

const API_URL: &str = "https://api.github.com/graphql";
const QUERY: &str = include_str!("query.graphql");
const SCHEMA: &str = include_str!("schema.sql");

#[derive(Debug, Clone)]
struct Config {
    database: PathBuf,
    page_size: u64,
    max_pages: u64,
    event_limit: u64,
}

#[derive(Debug)]
struct SearchResult {
    category: String,
    query: String,
    reported_count: u64,
    pages_fetched: u64,
    truncated: bool,
    ids: Vec<String>,
}

#[derive(Debug)]
struct Capture {
    captured_at: String,
    viewer: String,
    searches: Vec<SearchResult>,
    pull_requests: BTreeMap<String, Value>,
    request_count: u64,
    graphql_cost: u64,
    remaining: u64,
    reset_at: String,
}

struct GitHub {
    client: Client,
    token: String,
    requests: u64,
    cost: u64,
    remaining: u64,
    reset_at: String,
}

impl GitHub {
    fn new(token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("review-radar-data-spike")
            .build()?;
        Ok(Self {
            client,
            token,
            requests: 0,
            cost: 0,
            remaining: 0,
            reset_at: String::new(),
        })
    }

    fn query(&mut self, query: &str, variables: Value) -> Result<Value> {
        let response = self
            .client
            .post(API_URL)
            .bearer_auth(&self.token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .context("GitHub request failed")?
            .error_for_status()
            .context("GitHub returned an HTTP error")?;
        self.requests += 1;
        let body: Value = response.json().context("GitHub returned invalid JSON")?;
        if body.get("errors").is_some() {
            bail!("GitHub GraphQL returned errors; no capture was saved");
        }
        let data = body
            .get("data")
            .cloned()
            .ok_or_else(|| anyhow!("GitHub response omitted data"))?;
        if let Some(rate) = data.get("rateLimit") {
            self.cost += required_u64(rate, "cost")?;
            self.remaining = required_u64(rate, "remaining")?;
            self.reset_at = required_str(rate, "resetAt")?.to_owned();
        }
        Ok(data)
    }
}

fn main() -> Result<()> {
    let config = parse_config(env::args().skip(1))?;
    let token = env::var("GH_TOKEN").context("GH_TOKEN is required")?;
    let capture = collect(GitHub::new(token)?, &config)?;
    let capture_id = persist(&config, &capture)?;
    println!(
        "Saved capture {capture_id} with {} distinct PRs to {}",
        capture.pull_requests.len(),
        config.database.display()
    );
    for search in &capture.searches {
        println!(
            "  {}: {}/{} (truncated={})",
            search.category,
            search.ids.len(),
            search.reported_count,
            search.truncated
        );
    }
    println!(
        "  {} requests; GraphQL cost {}; {} points remaining",
        capture.request_count, capture.graphql_cost, capture.remaining
    );
    Ok(())
}

fn parse_config(args: impl Iterator<Item = String>) -> Result<Config> {
    let mut config = Config {
        database: "data/review-radar.sqlite3".into(),
            // Rich nested connections make larger pages time out at GitHub's
            // GraphQL edge. Keep every response below the verified threshold.
            page_size: 10,
        max_pages: 4,
        event_limit: 20,
    };
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow!("missing value for {flag}"))?;
        match flag.as_str() {
            "--database" => config.database = value.into(),
            "--page-size" => config.page_size = bounded(&value, 1, 100, &flag)?,
            "--max-pages" => config.max_pages = bounded(&value, 1, 10, &flag)?,
            "--event-limit" => config.event_limit = bounded(&value, 1, 50, &flag)?,
            _ => bail!("unknown argument: {flag}"),
        }
    }
    Ok(config)
}

fn bounded(value: &str, min: u64, max: u64, name: &str) -> Result<u64> {
    let value: u64 = value
        .parse()
        .with_context(|| format!("invalid value for {name}"))?;
    if !(min..=max).contains(&value) {
        bail!("{name} must be between {min} and {max}");
    }
    Ok(value)
}

fn collect(mut github: GitHub, config: &Config) -> Result<Capture> {
    let captured_at = Utc::now();
    let identity = github.query(
        "query { viewer { login } rateLimit { cost remaining resetAt } }",
        json!({}),
    )?;
    let viewer = required_str(required(&identity, "viewer")?, "login")?.to_owned();
    let cutoff =
        (captured_at - ChronoDuration::days(14)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let definitions = [
        (
            "review_requested",
            format!("is:pr is:open review-requested:{viewer}"),
        ),
        ("authored", format!("is:pr is:open author:{viewer}")),
        (
            "involved",
            format!("is:pr is:open involves:{viewer} -author:{viewer}"),
        ),
        (
            "review_involved",
            format!("is:pr is:open review-involves:{viewer} -author:{viewer}"),
        ),
        (
            "recent",
            format!("is:pr is:closed involves:{viewer} closed:>={cutoff}"),
        ),
        (
            "recent_review_involved",
            format!("is:pr is:closed review-involves:{viewer} closed:>={cutoff}"),
        ),
    ];
    let mut searches = Vec::new();
    let mut pull_requests = BTreeMap::new();
    for (category, base_query) in definitions {
        let query = format!("{base_query} sort:updated-desc");
        let mut cursor = Value::Null;
        let mut ids = Vec::new();
        let mut reported_count = 0;
        let mut pages_fetched = 0;
        let mut has_next_page = false;
        for _ in 0..config.max_pages {
            let data = github.query(
                QUERY,
                json!({
                    "search": query,
                    "pageSize": config.page_size,
                    "cursor": cursor,
                    "eventLimit": config.event_limit,
                }),
            )?;
            let search = required(&data, "search")?;
            reported_count = required_u64(search, "issueCount")?;
            for node in required_array(search, "nodes")? {
                if node.is_null() {
                    continue;
                }
                let id = required_str(node, "id")?.to_owned();
                if !ids.contains(&id) {
                    ids.push(id.clone());
                }
                pull_requests.insert(id, node.clone());
            }
            pages_fetched += 1;
            let page_info = required(search, "pageInfo")?;
            has_next_page = required_bool(page_info, "hasNextPage")?;
            if !has_next_page {
                break;
            }
            cursor = page_info
                .get("endCursor")
                .cloned()
                .filter(|v| !v.is_null())
                .ok_or_else(|| anyhow!("GitHub reported another page without a cursor"))?;
        }
        let truncated = has_next_page || reported_count > ids.len() as u64;
        searches.push(SearchResult {
            category: category.into(),
            query,
            reported_count,
            pages_fetched,
            truncated,
            ids,
        });
    }
    Ok(Capture {
        captured_at: captured_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        viewer,
        searches,
        pull_requests,
        request_count: github.requests,
        graphql_cost: github.cost,
        remaining: github.remaining,
        reset_at: github.reset_at,
    })
}

fn persist(config: &Config, capture: &Capture) -> Result<i64> {
    if let Some(parent) = config.database.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut connection = Connection::open(&config.database)?;
    connection.execute_batch(SCHEMA)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO captures (captured_at, viewer_login, page_size, max_pages, event_limit, request_count, graphql_cost, rate_limit_remaining, rate_limit_reset_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![capture.captured_at, capture.viewer, config.page_size, config.max_pages, config.event_limit, capture.request_count, capture.graphql_cost, capture.remaining, capture.reset_at],
    )?;
    let capture_id = transaction.last_insert_rowid();
    for (id, pr) in &capture.pull_requests {
        transaction.execute(
            "INSERT INTO pull_requests (capture_id, node_id, repository, number, title, url, state, is_draft, author_login, created_at, updated_at, closed_at, merged_at, review_decision, mergeable, merge_state_status, payload) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![capture_id, id, required_str(required(pr, "repository")?, "nameWithOwner")?, required_u64(pr, "number")?, required_str(pr, "title")?, required_str(pr, "url")?, required_str(pr, "state")?, required_bool(pr, "isDraft")?, optional_nested_str(pr, &["author", "login"]), required_str(pr, "createdAt")?, required_str(pr, "updatedAt")?, optional_str(pr, "closedAt"), optional_str(pr, "mergedAt"), optional_str(pr, "reviewDecision"), required_str(pr, "mergeable")?, required_str(pr, "mergeStateStatus")?, serde_json::to_string(pr)?],
        )?;
    }
    for search in &capture.searches {
        transaction.execute(
            "INSERT INTO searches (capture_id, category, query, reported_count, fetched_count, pages_fetched, truncated) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![capture_id, search.category, search.query, search.reported_count, search.ids.len(), search.pages_fetched, search.truncated],
        )?;
        for id in &search.ids {
            transaction.execute(
                "INSERT INTO search_memberships (capture_id, category, node_id) VALUES (?, ?, ?)",
                params![capture_id, search.category, id],
            )?;
        }
    }
    transaction.commit()?;
    Ok(capture_id)
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| anyhow!("GitHub response omitted {key}"))
}
fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    required(value, key)?
        .as_str()
        .ok_or_else(|| anyhow!("GitHub response has invalid {key}"))
}
fn required_u64(value: &Value, key: &str) -> Result<u64> {
    required(value, key)?
        .as_u64()
        .ok_or_else(|| anyhow!("GitHub response has invalid {key}"))
}
fn required_bool(value: &Value, key: &str) -> Result<bool> {
    required(value, key)?
        .as_bool()
        .ok_or_else(|| anyhow!("GitHub response has invalid {key}"))
}
fn required_array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    required(value, key)?
        .as_array()
        .ok_or_else(|| anyhow!("GitHub response has invalid {key}"))
}
fn optional_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}
fn optional_nested_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |node, key| node.get(key))?
        .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_bounded() {
        assert_eq!(parse_config(std::iter::empty()).unwrap().page_size, 10);
        assert!(parse_config(["--page-size".into(), "101".into()].into_iter()).is_err());
        assert!(parse_config(["--max-pages".into(), "0".into()].into_iter()).is_err());
        let config = parse_config(["--event-limit".into(), "5".into()].into_iter()).unwrap();
        assert_eq!(config.event_limit, 5);
    }

    #[test]
    fn schema_preserves_snapshots_and_memberships() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(SCHEMA).unwrap();
        let json = r#"{"id":"PR_1"}"#;
        connection
            .execute(
                "INSERT INTO captures VALUES (1, 'now', 'alice', 25, 4, 20, 2, 3, 4997, 'later')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO searches VALUES (1, 'authored', 'query', 1, 1, 1, 0)",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO pull_requests VALUES (1, 'PR_1', 'org/repo', 1, 'Title', 'https://example.test', 'OPEN', 0, 'alice', 'then', 'now', NULL, NULL, NULL, 'UNKNOWN', 'UNKNOWN', ?)", [json]).unwrap();
        connection
            .execute(
                "INSERT INTO search_memberships VALUES (1, 'authored', 'PR_1')",
                [],
            )
            .unwrap();
        let count: i64 = connection
            .query_row("SELECT count(*) FROM search_memberships", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
