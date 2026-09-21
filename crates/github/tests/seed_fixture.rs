use std::{collections::BTreeSet, fs, path::PathBuf};

use review_radar_domain::{ranking::by_id, Snapshot, WorkspaceView};
use serde_json::Value;

fn seed() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/github/github-snapshot.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn generalized_seed_retains_real_capture_shape() {
    let seed = seed();
    assert_eq!(seed["schemaVersion"], 1);
    assert_eq!(seed["provenance"], "generalized-real-capture");
    assert_eq!(seed["viewer"]["login"], "user-001");

    let prs = seed["pullRequests"].as_array().unwrap();
    let searches = seed["searches"].as_array().unwrap();
    assert_eq!(prs.len(), 97);
    assert_eq!(searches.len(), 6);

    let ids = prs
        .iter()
        .map(|pr| pr["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), prs.len());
    for search in searches {
        assert!(!search["truncated"].as_bool().unwrap());
        assert_eq!(
            search["fetchedCount"].as_u64().unwrap(),
            search["ids"].as_array().unwrap().len() as u64
        );
        for id in search["ids"].as_array().unwrap() {
            assert!(ids.contains(id.as_str().unwrap()));
        }
    }

    let states = values(prs, "state");
    assert!(states.is_superset(&set(["OPEN", "MERGED", "CLOSED"])));
    let decisions = values(prs, "reviewDecision");
    assert!(decisions.is_superset(&set(["REVIEW_REQUIRED", "CHANGES_REQUESTED", "APPROVED",])));
    let mergeability = values(prs, "mergeable");
    assert!(mergeability.is_superset(&set(["MERGEABLE", "CONFLICTING", "UNKNOWN"])));
    assert!(prs.iter().any(|pr| pr["isDraft"] == true));
    assert!(prs.iter().any(|pr| {
        pr.pointer("/commits/nodes/0/commit/statusCheckRollup/state")
            == Some(&Value::String("FAILURE".into()))
    }));
}

#[test]
fn every_identity_field_is_generalized() {
    visit(&seed(), "");
}

#[test]
fn fixture_projects_each_supported_workspace_view() {
    let snapshot = Snapshot::from_json(&fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/github/github-snapshot.json"),
    )
    .unwrap())
    .unwrap();
    let ranking = by_id("tailored").unwrap();

    for view in [
        WorkspaceView::Tailored,
        WorkspaceView::Action,
        WorkspaceView::MyPrs,
        WorkspaceView::Following,
        WorkspaceView::Recent,
    ] {
        let cards = snapshot.view_with_ranking(view, ranking.as_ref());
        let ids = cards.iter().map(|card| &card.id).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), cards.len(), "duplicate cards in {view:?}");
        assert!(!cards.iter().any(|card| card.id.is_empty()));
        if view == WorkspaceView::Action {
            assert!(cards.iter().all(|card| card.attention_required));
        }
    }
}

fn visit(value: &Value, key: &str) {
    match value {
        Value::Object(object) => {
            for (child_key, child) in object {
                visit(child, child_key);
            }
        }
        Value::Array(values) => {
            for child in values {
                visit(child, key);
            }
        }
        Value::String(value) => match key {
            "id" => assert!(value.starts_with("node-"), "non-generalized ID: {value}"),
            "oid" => assert!(
                value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "non-generalized commit SHA"
            ),
            "login" => assert!(value.starts_with("user-"), "non-generalized login: {value}"),
            "slug" => assert!(value.starts_with("team-"), "non-generalized team: {value}"),
            "nameWithOwner" => assert!(
                value.starts_with("example/repository-"),
                "non-generalized repository: {value}"
            ),
            "name" | "context" => assert!(
                value.starts_with("check-"),
                "non-generalized check name: {value}"
            ),
            "title" => assert!(
                value.starts_with("Example pull request "),
                "non-generalized title: {value}"
            ),
            "url" => assert!(
                value.starts_with("https://github.com/example/repository-"),
                "non-generalized URL: {value}"
            ),
            _ => {}
        },
        _ => {}
    }
}

fn values<'a>(prs: &'a [Value], key: &str) -> BTreeSet<&'a str> {
    prs.iter().filter_map(|pr| pr[key].as_str()).collect()
}

fn set<const N: usize>(values: [&'static str; N]) -> BTreeSet<&'static str> {
    values.into_iter().collect()
}
