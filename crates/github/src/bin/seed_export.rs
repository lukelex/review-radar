use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
};

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

#[derive(Debug)]
struct Config {
    database: PathBuf,
    output: PathBuf,
    capture_id: Option<i64>,
}

#[derive(Default)]
struct Aliases {
    values: BTreeMap<&'static str, BTreeMap<String, String>>,
}

impl Aliases {
    fn alias(&mut self, kind: &'static str, original: &str) -> String {
        let values = self.values.entry(kind).or_default();
        let next = values.len() + 1;
        values
            .entry(original.to_owned())
            .or_insert_with(|| format!("{kind}-{next:03}"))
            .clone()
    }
}

fn main() -> Result<()> {
    let config = parse_config(env::args().skip(1))?;
    let connection =
        Connection::open_with_flags(&config.database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("cannot open {}", config.database.display()))?;
    let capture_id = match config.capture_id {
        Some(id) => id,
        None => connection
            .query_row("SELECT max(id) FROM captures", [], |row| row.get(0))
            .context("database contains no captures")?,
    };
    let seed = export_seed(&connection, capture_id)?;
    let encoded = serde_json::to_string_pretty(&seed)? + "\n";
    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config.output, encoded)?;
    println!(
        "Exported capture {capture_id} as generalized seed {}",
        config.output.display()
    );
    Ok(())
}

fn parse_config(args: impl Iterator<Item = String>) -> Result<Config> {
    let mut config = Config {
        database: "/data/review-radar.sqlite3".into(),
        output: "/fixtures/github-snapshot.json".into(),
        capture_id: None,
    };
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow!("missing value for {flag}"))?;
        match flag.as_str() {
            "--database" => config.database = value.into(),
            "--output" => config.output = value.into(),
            "--capture-id" => {
                config.capture_id = Some(value.parse().context("invalid capture ID")?)
            }
            _ => bail!("unknown argument: {flag}"),
        }
    }
    Ok(config)
}

fn export_seed(connection: &Connection, capture_id: i64) -> Result<Value> {
    let (captured_at, viewer, page_size, max_pages, event_limit): (String, String, i64, i64, i64) =
        connection
            .query_row(
                "SELECT captured_at, viewer_login, page_size, max_pages, event_limit FROM captures WHERE id = ?",
                [capture_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .with_context(|| format!("capture {capture_id} does not exist"))?;

    let mut aliases = Aliases::default();
    let sanitized_viewer = aliases.alias("user", &viewer);
    let mut pr_ids = BTreeMap::new();
    let mut pull_requests = Vec::new();
    let mut statement = connection.prepare(
        "SELECT node_id, payload FROM pull_requests WHERE capture_id = ? ORDER BY node_id",
    )?;
    let rows = statement.query_map([capture_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (original_id, payload) = row?;
        let id = aliases.alias("node", &original_id);
        pr_ids.insert(original_id, id.clone());
        let mut payload: Value = serde_json::from_str(&payload)?;
        sanitize_value(&mut payload, "", &mut aliases);
        let repository = payload
            .pointer("/repository/nameWithOwner")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("sanitized PR omitted repository"))?
            .to_owned();
        let number = pull_requests.len() + 1;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| anyhow!("PR payload is not an object"))?;
        object.insert("id".into(), Value::String(id));
        object.insert("number".into(), json!(number));
        object.insert(
            "title".into(),
            Value::String(format!("Example pull request {number:03}")),
        );
        object.insert(
            "url".into(),
            Value::String(format!("https://github.com/{repository}/pull/{number}")),
        );
        pull_requests.push(payload);
    }

    let mut searches = Vec::new();
    let mut statement = connection.prepare(
        "SELECT category, query, reported_count, fetched_count, pages_fetched, truncated
         FROM searches WHERE capture_id = ? ORDER BY category",
    )?;
    let rows = statement.query_map([capture_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, bool>(5)?,
        ))
    })?;
    for row in rows {
        let (category, query, reported_count, fetched_count, pages_fetched, truncated) = row?;
        let mut membership_statement = connection.prepare(
            "SELECT node_id FROM search_memberships WHERE capture_id = ? AND category = ? ORDER BY node_id",
        )?;
        let original_ids = membership_statement
            .query_map(params![capture_id, category], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let ids = original_ids
            .iter()
            .map(|id| {
                pr_ids
                    .get(id)
                    .cloned()
                    .ok_or_else(|| anyhow!("membership references unknown PR"))
            })
            .collect::<Result<Vec<_>>>()?;
        searches.push(json!({
            "category": category,
            "query": query.replace(&viewer, &sanitized_viewer),
            "reportedCount": reported_count,
            "fetchedCount": fetched_count,
            "pagesFetched": pages_fetched,
            "truncated": truncated,
            "ids": ids,
        }));
    }

    let seed = json!({
        "schemaVersion": 1,
        "provenance": "generalized-real-capture",
        "capturedAt": captured_at,
        "viewer": { "login": sanitized_viewer },
        "limits": {
            "pageSize": page_size,
            "maxPagesPerSearch": max_pages,
            "eventLimit": event_limit,
            "reviewRequests": 100,
            "checkContexts": 100,
        },
        "searches": searches,
        "pullRequests": pull_requests,
    });
    validate_seed(&seed)?;
    Ok(seed)
}

fn sanitize_value(value: &mut Value, key: &str, aliases: &mut Aliases) {
    match value {
        Value::Object(object) => {
            for (child_key, child) in object {
                sanitize_value(child, child_key, aliases);
            }
        }
        Value::Array(values) => {
            for child in values {
                sanitize_value(child, key, aliases);
            }
        }
        Value::String(original) => {
            let replacement = match key {
                "id" => Some(aliases.alias("node", original)),
                "oid" => {
                    let index = aliases.alias("commit", original);
                    let number = index.rsplit('-').next().unwrap_or("0");
                    Some(format!("{number:0>40}"))
                }
                "login" => Some(aliases.alias("user", original)),
                "slug" => Some(aliases.alias("team", original)),
                "nameWithOwner" => {
                    Some(format!("example/{}", aliases.alias("repository", original)))
                }
                "name" | "context" => Some(aliases.alias("check", original)),
                "title" => Some(aliases.alias("title", original)),
                "url" => Some("https://github.com/example/repository/pull/1".into()),
                _ => None,
            };
            if let Some(replacement) = replacement {
                *original = replacement;
            }
        }
        _ => {}
    }
}

fn validate_seed(seed: &Value) -> Result<()> {
    let viewer = seed.pointer("/viewer/login").and_then(Value::as_str);
    if viewer != Some("user-001") {
        bail!("seed viewer must be user-001");
    }
    let prs = seed
        .get("pullRequests")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("seed omitted pull requests"))?;
    let ids = prs
        .iter()
        .map(|pr| {
            let object = pr.as_object().ok_or_else(|| anyhow!("invalid PR"))?;
            validate_pr(object)?;
            Ok(object.get("id").and_then(Value::as_str).unwrap().to_owned())
        })
        .collect::<Result<BTreeSet<_>>>()?;
    for search in seed
        .get("searches")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("seed omitted searches"))?
    {
        for id in search
            .get("ids")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            if !id.as_str().is_some_and(|id| ids.contains(id)) {
                bail!("search membership references unknown generalized PR");
            }
        }
    }
    Ok(())
}

fn validate_pr(pr: &Map<String, Value>) -> Result<()> {
    let id = pr.get("id").and_then(Value::as_str).unwrap_or_default();
    let title = pr.get("title").and_then(Value::as_str).unwrap_or_default();
    let repository = pr
        .get("repository")
        .and_then(Value::as_object)
        .and_then(|repo| repo.get("nameWithOwner"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let url = pr.get("url").and_then(Value::as_str).unwrap_or_default();
    if !id.starts_with("node-")
        || !title.starts_with("Example pull request ")
        || !repository.starts_with("example/repository-")
        || !url.starts_with("https://github.com/example/repository-")
    {
        bail!("seed contains a non-generalized PR identity");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursively_generalizes_identity_fields() {
        let mut value = json!({
            "id": "PR_private",
            "author": { "login": "alice" },
            "repository": { "nameWithOwner": "company/secret" },
            "commit": { "oid": "deadbeef", "status": { "context": "company-ci" } },
            "reviewer": { "slug": "secret-team", "organization": { "login": "company" } },
            "state": "OPEN"
        });
        sanitize_value(&mut value, "", &mut Aliases::default());
        let encoded = value.to_string();
        for private in ["PR_private", "alice", "company", "secret", "deadbeef"] {
            assert!(!encoded.contains(private));
        }
        assert_eq!(value["state"], "OPEN");
        assert_eq!(value["author"]["login"], "user-001");
    }
}
