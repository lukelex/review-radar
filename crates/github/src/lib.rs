use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    io::{self, Write},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, info, info_span, warn};
use tracing_subscriber::EnvFilter;

pub mod projection;

const API_URL: &str = "https://api.github.com/graphql";
const SEARCH_QUERY: &str = include_str!("search.graphql");
const HYDRATE_QUERY: &str = include_str!("hydrate.graphql");
const SCHEMA: &str = include_str!("schema.sql");
const INITIAL_HYDRATION_BATCH: usize = 6;
const MIN_HYDRATION_BATCH: usize = 2;
// Ten is the existing verified upper bound for nested hydration responses.
const MAX_HYDRATION_BATCH: usize = 10;
const MAX_REQUEST_ATTEMPTS: usize = 3;
const REQUEST_BACKOFF: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct Config {
    pub database: PathBuf,
    pub page_size: u64,
    pub max_pages: u64,
    pub event_limit: u64,
}

#[derive(Debug)]
pub struct SearchResult {
    pub category: String,
    pub query: String,
    pub reported_count: u64,
    pub pages_fetched: u64,
    pub truncated: bool,
    pub ids: Vec<String>,
}

struct SearchWork {
    search: SearchResult,
    pub pull_requests: BTreeMap<String, Value>,
    github: GitHub,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchData {
    search: SearchConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchConnection {
    issue_count: u64,
    page_info: PageInfo,
    nodes: Vec<Option<SearchNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchNode {
    id: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug)]
pub struct Capture {
    pub captured_at: String,
    pub viewer: String,
    pub searches: Vec<SearchResult>,
    pull_requests: BTreeMap<String, Value>,
    pub request_count: u64,
    pub graphql_cost: u64,
    pub remaining: u64,
    pub reset_at: String,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub request_time_ms: u128,
    pub slowest_request_ms: u128,
    pub response_bytes: u64,
    pub stale_ids: Vec<String>,
}

struct GitHub {
    client: Client,
    token: String,
    requests: u64,
    cost: u64,
    remaining: u64,
    reset_at: String,
    request_time: Duration,
    slowest_request: Duration,
    response_bytes: u64,
}

impl GitHub {
    fn new(token: String) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
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
            request_time: Duration::ZERO,
            slowest_request: Duration::ZERO,
            response_bytes: 0,
        })
    }

    fn query(&mut self, query: &str, variables: Value) -> Result<Value> {
        let query_kind = query_kind(query);
        let _span = info_span!("github.request", query = query_kind).entered();
        if self.requests > 0 && self.remaining == 0 {
            bail!(
                "GitHub rate limit exhausted; retry after {}",
                if self.reset_at.is_empty() {
                    "the reset time"
                } else {
                    &self.reset_at
                }
            );
        }
        let mut attempt = 0;
        let response = loop {
            attempt += 1;
            let started = Instant::now();
            let response = self
                .client
                .post(API_URL)
                .bearer_auth(&self.token)
                .json(&json!({ "query": query, "variables": &variables }))
                .send();
            let elapsed = started.elapsed();
            self.request_time += elapsed;
            self.slowest_request = self.slowest_request.max(elapsed);
            match response {
                Ok(response)
                    if retryable_status(response.status().as_u16())
                        && attempt < MAX_REQUEST_ATTEMPTS =>
                {
                    warn!(
                        status = response.status().as_u16(),
                        attempt,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "retrying GitHub request"
                    );
                    thread::sleep(REQUEST_BACKOFF * attempt as u32);
                }
                Ok(response) => break response,
                Err(error) if error.is_timeout() && attempt < MAX_REQUEST_ATTEMPTS => {
                    warn!(
                        attempt,
                        elapsed_ms = elapsed.as_millis() as u64,
                        "GitHub request timed out; retrying"
                    );
                    thread::sleep(REQUEST_BACKOFF * attempt as u32);
                }
                Err(error) => {
                    let category = if error.is_timeout() {
                        "timeout"
                    } else {
                        "transport"
                    };
                    return Err(anyhow!("GitHub request {category} failed: {error}"));
                }
            }
        };
        debug!(
            status = response.status().as_u16(),
            attempt, "GitHub request completed"
        );
        if matches!(response.status().as_u16(), 403 | 429) {
            bail!(
                "GitHub rate limit request rejected (HTTP {}); retry after {}",
                response.status(),
                if self.reset_at.is_empty() {
                    "the reset time"
                } else {
                    &self.reset_at
                }
            );
        }
        let response = response
            .error_for_status()
            .context("GitHub returned an HTTP error")?;
        self.requests += 1;
        let body_bytes = response
            .bytes()
            .context("GitHub response body could not be read")?;
        self.response_bytes += body_bytes.len() as u64;
        let body: Value =
            serde_json::from_slice(&body_bytes).context("GitHub returned invalid JSON")?;
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

fn query_kind(query: &str) -> &'static str {
    if query == HYDRATE_QUERY {
        "hydrate"
    } else if query == SEARCH_QUERY {
        "search"
    } else {
        "metadata"
    }
}

fn retryable_status(status: u16) -> bool {
    status == 408 || status == 429 || (500..=599).contains(&status)
}

pub fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("review_radar_github=info")),
        )
        .with_target(false)
        .init();
    let config = parse_config(env::args().skip(1))?;
    let token = env::var("GH_TOKEN").context("GH_TOKEN is required")?;
    let cache = load_cache(&config.database)?;
    let capture = collect(GitHub::new(token)?, &config, &cache)?;
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
        "  {} requests; GraphQL cost {}; {} points remaining (reset {})",
        capture.request_count, capture.graphql_cost, capture.remaining, capture.reset_at
    );
    println!(
        "  cache: {} hits, {} hydrations; requests took {} ms total (slowest {} ms)",
        capture.cache_hits,
        capture.cache_misses,
        capture.request_time_ms,
        capture.slowest_request_ms
    );
    println!("  response data: {} bytes", capture.response_bytes);
    if !capture.stale_ids.is_empty() {
        println!(
            "  stale hydration fallback: {} PRs",
            capture.stale_ids.len()
        );
    }
    Ok(())
}

fn report_progress(message: impl AsRef<str>) {
    println!("Progress: {}", message.as_ref());
    let _ = io::stdout().flush();
}

pub fn parse_config(args: impl Iterator<Item = String>) -> Result<Config> {
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

fn hydration_batch_size(remaining: u64) -> usize {
    if remaining < 250 {
        MIN_HYDRATION_BATCH
    } else if remaining < 500 {
        4
    } else {
        INITIAL_HYDRATION_BATCH
    }
}

fn next_hydration_batch_size(
    current: usize,
    elapsed: Duration,
    cost: u64,
    remaining: u64,
) -> usize {
    // Leave room for the next request and avoid increasing batches near the
    // rate-limit floor. The latency and cost limits are deliberately
    // conservative because hydration includes nested review history.
    if remaining < 250 || elapsed > Duration::from_secs(30) || cost > 500 {
        return (current / 2).max(MIN_HYDRATION_BATCH);
    }
    if elapsed <= Duration::from_secs(10) && cost <= 300 && remaining >= 500 {
        return (current + 2).min(MAX_HYDRATION_BATCH);
    }
    current.clamp(MIN_HYDRATION_BATCH, MAX_HYDRATION_BATCH)
}

fn collect(
    mut github: GitHub,
    config: &Config,
    cache: &BTreeMap<String, Value>,
) -> Result<Capture> {
    let captured_at = Utc::now();
    let identity = github.query(
        "query { viewer { login } rateLimit { cost remaining resetAt } }",
        json!({}),
    )?;
    let viewer = required_str(required(&identity, "viewer")?, "login")?.to_owned();
    let cutoff =
        (captured_at - ChronoDuration::days(14)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let definitions = vec![
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
    let search_count = definitions.len();
    let parallel = github.remaining >= search_count as u64;
    let mut work = if parallel {
        let token = github.token.clone();
        let client = github.client.clone();
        let remaining = github.remaining;
        let reset_at = github.reset_at.clone();
        std::thread::scope(|scope| {
            let handles = definitions.into_iter().enumerate().map(
                |(search_index, (category, base_query))| {
                    let token = token.clone();
                    let client = client.clone();
                    let reset_at = reset_at.clone();
                    scope.spawn(move || {
                        let worker = GitHub {
                            client,
                            token,
                            requests: 0,
                            cost: 0,
                            remaining,
                            reset_at,
                            request_time: Duration::ZERO,
                            slowest_request: Duration::ZERO,
                            response_bytes: 0,
                        };
                        collect_search(
                            worker,
                            config,
                            search_index,
                            search_count,
                            category,
                            base_query,
                        )
                    })
                },
            );
            handles
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| anyhow!("search worker panicked"))?
                })
                .collect::<Result<Vec<_>>>()
        })?
    } else {
        let token = github.token.clone();
        let client = github.client.clone();
        let remaining = github.remaining;
        let reset_at = github.reset_at.clone();
        definitions
            .into_iter()
            .enumerate()
            .map(|(search_index, (category, base_query))| {
                collect_search(
                    GitHub {
                        client: client.clone(),
                        token: token.clone(),
                        requests: 0,
                        cost: 0,
                        remaining,
                        reset_at: reset_at.clone(),
                        request_time: Duration::ZERO,
                        slowest_request: Duration::ZERO,
                        response_bytes: 0,
                    },
                    config,
                    search_index,
                    search_count,
                    category,
                    base_query,
                )
            })
            .collect::<Result<Vec<_>>>()?
    };
    work.sort_by_key(|(index, _)| *index);
    for (_, result) in work {
        let SearchWork {
            search,
            pull_requests: found,
            github: worker,
        } = result;
        searches.push(search);
        pull_requests.extend(found);
        github.requests += worker.requests;
        github.cost += worker.cost;
        github.remaining = github.remaining.min(worker.remaining);
        if !worker.reset_at.is_empty() {
            github.reset_at = worker.reset_at;
        }
        github.request_time += worker.request_time;
        github.slowest_request = github.slowest_request.max(worker.slowest_request);
        github.response_bytes += worker.response_bytes;
    }

    let ids_to_hydrate: Vec<String> = pull_requests
        .iter()
        .filter_map(|(id, summary)| {
            let cached = cache.get(id)?;
            (required_str(summary, "updatedAt").ok()? != required_str(cached, "updatedAt").ok()?)
                .then_some(id.clone())
        })
        .chain(pull_requests.iter().filter_map(|(id, summary)| {
            (!cache.contains_key(id) && summary.get("updatedAt").is_some()).then_some(id.clone())
        }))
        .collect();

    let cache_hits = pull_requests.len() as u64 - ids_to_hydrate.len() as u64;
    let cache_misses = ids_to_hydrate.len() as u64;
    info!(cache_hits, cache_misses, "prepared pull-request hydration");
    let mut hydrated = BTreeMap::new();
    let mut hydration_error = None;
    let mut batch_size = hydration_batch_size(github.remaining);
    let mut offset = 0;
    let mut batch_index = 0;
    while offset < ids_to_hydrate.len() {
        let end = (offset + batch_size).min(ids_to_hydrate.len());
        let ids = &ids_to_hydrate[offset..end];
        report_progress(format!(
            "hydrating batch {} ({} PRs; batch size {})",
            batch_index + 1,
            ids.len(),
            batch_size
        ));
        let request_cost = github.cost;
        let request_time = github.request_time;
        let data = match github.query(
            HYDRATE_QUERY,
            json!({"ids": ids, "eventLimit": config.event_limit}),
        ) {
            Ok(data) => data,
            Err(error) => {
                hydration_error = Some(error);
                break;
            }
        };
        batch_size = next_hydration_batch_size(
            batch_size,
            github.request_time.saturating_sub(request_time),
            github.cost.saturating_sub(request_cost),
            github.remaining,
        );
        for node in required_array(&data, "nodes")? {
            if !node.is_null() {
                hydrated.insert(required_str(node, "id")?.to_owned(), node.clone());
            }
        }
        offset = end;
        batch_index += 1;
    }
    let mut stale_ids = Vec::new();
    for (id, summary) in &mut pull_requests {
        if let Some(node) = hydrated.remove(id) {
            *summary = node;
        } else if let Some(cached) = cache.get(id) {
            *summary = cached.clone();
            mark_hydration_stale(summary);
            stale_ids.push(id.clone());
        } else {
            return Err(hydration_error.unwrap_or_else(|| {
                anyhow!("GitHub did not return pull request {id} during hydration")
            }));
        }
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
        cache_hits,
        cache_misses,
        request_time_ms: github.request_time.as_millis(),
        slowest_request_ms: github.slowest_request.as_millis(),
        response_bytes: github.response_bytes,
        stale_ids,
    })
}

/// Collect a complete GitHub snapshot using the supplied cache.
///
/// This is the reusable collector boundary for native clients. It owns the
/// authenticated HTTP client and returns only a complete capture or an error;
/// callers never observe a partially hydrated projection.
pub fn collect_with_token(
    token: impl Into<String>,
    config: &Config,
    cache: &BTreeMap<String, Value>,
) -> Result<Capture> {
    collect(GitHub::new(token.into())?, config, cache)
}

fn collect_search(
    mut github: GitHub,
    config: &Config,
    search_index: usize,
    search_count: usize,
    category: &str,
    base_query: String,
) -> Result<(usize, SearchWork)> {
    let query = format!("{base_query} sort:updated-desc");
    let mut cursor: Option<String> = None;
    let mut ids = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut pull_requests = BTreeMap::new();
    let mut reported_count = 0;
    let mut pages_fetched = 0;
    let mut has_next_page = false;
    for _ in 0..config.max_pages {
        let data = github.query(
            SEARCH_QUERY,
            json!({
                "search": query,
                "pageSize": config.page_size,
                "cursor": cursor,
                "eventLimit": config.event_limit,
            }),
        )?;
        let response: SearchData = serde_json::from_value(data)
            .context("GitHub search response did not match the expected schema")?;
        let search = response.search;
        reported_count = search.issue_count;
        for node in search.nodes.into_iter().flatten() {
            let id = node.id;
            if seen_ids.insert(id.clone()) {
                ids.push(id.clone());
            }
            pull_requests
                .entry(id.clone())
                .or_insert_with(|| json!({"id": id, "updatedAt": node.updated_at}));
        }
        pages_fetched += 1;
        report_progress(format!(
            "search {}/{} · {} · page {}/{}",
            search_index + 1,
            search_count,
            category,
            pages_fetched,
            config.max_pages
        ));
        has_next_page = search.page_info.has_next_page;
        if !has_next_page {
            break;
        }
        cursor = search.page_info.end_cursor;
        if cursor.is_none() {
            return Err(anyhow!("GitHub reported another page without a cursor"));
        }
    }
    let truncated = has_next_page || reported_count > ids.len() as u64;
    Ok((
        search_index,
        SearchWork {
            search: SearchResult {
                category: category.into(),
                query,
                reported_count,
                pages_fetched,
                truncated,
                ids,
            },
            pull_requests,
            github,
        },
    ))
}

fn mark_hydration_stale(payload: &mut Value) {
    if let Some(object) = payload.as_object_mut() {
        object.insert("_reviewRadar".into(), json!({"hydrationStale": true}));
    }
}

fn load_cache(database: &PathBuf) -> Result<BTreeMap<String, Value>> {
    if !database.exists() {
        return Ok(BTreeMap::new());
    }
    let connection = Connection::open(database)?;
    connection.execute_batch(SCHEMA)?;
    let mut statement = connection.prepare(
        "SELECT node_id, payload FROM pull_requests WHERE capture_id = (SELECT max(id) FROM captures)",
    )?;
    let rows = statement.query_map([], |row| {
        let id: String = row.get(0)?;
        let payload: String = row.get(1)?;
        Ok((id, payload))
    })?;
    let mut cache = BTreeMap::new();
    for row in rows {
        let (id, payload) = row?;
        let mut payload: Value = serde_json::from_str(&payload)?;
        let object = payload
            .as_object_mut()
            .context("stored PR payload is not an object")?;
        // `node_id` is the database's canonical identity. Older captures may
        // not have retained the redundant field in their JSON payload.
        object.insert("id".into(), Value::String(id.clone()));
        cache.insert(id, payload);
    }
    Ok(cache)
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
    fn hydration_keeps_check_summary_and_truncation_without_nested_check_payloads() {
        assert!(HYDRATE_QUERY.contains("statusCheckRollup"));
        assert!(HYDRATE_QUERY.contains("contexts(first: 100)"));
        assert!(HYDRATE_QUERY.contains("totalCount"));
        assert!(HYDRATE_QUERY.contains("hasNextPage"));
        assert!(!HYDRATE_QUERY.contains("... on CheckRun"));
        assert!(!HYDRATE_QUERY.contains("... on StatusContext"));
        assert!(!HYDRATE_QUERY.contains("reviewRequests"));
        assert!(HYDRATE_QUERY.contains("REVIEW_REQUESTED_EVENT"));
        assert!(HYDRATE_QUERY.contains("REVIEW_REQUEST_REMOVED_EVENT"));
        assert!(HYDRATE_QUERY.contains("HEAD_REF_FORCE_PUSHED_EVENT"));
        assert!(HYDRATE_QUERY.contains("handoffItems: timelineItems"));
        assert!(HYDRATE_QUERY.contains("timelineItems(last: $eventLimit, itemTypes: [READY_FOR_REVIEW_EVENT"));
        assert!(HYDRATE_QUERY.contains("requestedReviewer"));
        assert!(HYDRATE_QUERY.contains("afterCommit { oid }"));
        assert!(!HYDRATE_QUERY.contains("isResolved"));
        assert!(!HYDRATE_QUERY.contains("reviews(last: $eventLimit) {\n        totalCount"));
    }

    #[test]
    fn hydration_batches_adapt_with_rate_limit_and_request_pressure() {
        assert_eq!(hydration_batch_size(1_000), INITIAL_HYDRATION_BATCH);
        assert_eq!(hydration_batch_size(400), 4);
        assert_eq!(hydration_batch_size(100), MIN_HYDRATION_BATCH);

        assert_eq!(
            next_hydration_batch_size(6, Duration::from_secs(5), 200, 1_000),
            8
        );
        assert_eq!(
            next_hydration_batch_size(10, Duration::from_secs(31), 200, 1_000),
            5
        );
        assert_eq!(
            next_hydration_batch_size(3, Duration::from_secs(31), 200, 1_000),
            MIN_HYDRATION_BATCH
        );
        assert_eq!(
            next_hydration_batch_size(10, Duration::from_secs(5), 200, 200),
            5
        );
        assert_eq!(
            next_hydration_batch_size(20, Duration::from_secs(5), 200, 1_000),
            MAX_HYDRATION_BATCH
        );
    }

    #[test]
    fn exhausted_rate_limit_stops_follow_up_requests_with_retry_guidance() {
        let mut github = GitHub::new("token".into()).unwrap();
        github.requests = 1;
        github.remaining = 0;
        github.reset_at = "2026-09-21T23:00:00Z".into();
        let error = github.query("query", json!({})).unwrap_err().to_string();
        assert!(error.contains("rate limit exhausted"));
        assert!(error.contains("2026-09-21T23:00:00Z"));
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
