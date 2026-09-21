PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS captures (
  id INTEGER PRIMARY KEY,
  captured_at TEXT NOT NULL,
  viewer_login TEXT NOT NULL,
  page_size INTEGER NOT NULL,
  max_pages INTEGER NOT NULL,
  event_limit INTEGER NOT NULL,
  request_count INTEGER NOT NULL,
  graphql_cost INTEGER NOT NULL,
  rate_limit_remaining INTEGER NOT NULL,
  rate_limit_reset_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS searches (
  capture_id INTEGER NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
  category TEXT NOT NULL,
  query TEXT NOT NULL,
  reported_count INTEGER NOT NULL,
  fetched_count INTEGER NOT NULL,
  pages_fetched INTEGER NOT NULL,
  truncated INTEGER NOT NULL CHECK (truncated IN (0, 1)),
  PRIMARY KEY (capture_id, category)
);

CREATE TABLE IF NOT EXISTS pull_requests (
  capture_id INTEGER NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
  node_id TEXT NOT NULL,
  repository TEXT NOT NULL,
  number INTEGER NOT NULL,
  title TEXT NOT NULL,
  url TEXT NOT NULL,
  state TEXT NOT NULL,
  is_draft INTEGER NOT NULL CHECK (is_draft IN (0, 1)),
  author_login TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  merged_at TEXT,
  review_decision TEXT,
  mergeable TEXT NOT NULL,
  merge_state_status TEXT NOT NULL,
  payload TEXT NOT NULL CHECK (json_valid(payload)),
  PRIMARY KEY (capture_id, node_id)
);

CREATE TABLE IF NOT EXISTS search_memberships (
  capture_id INTEGER NOT NULL,
  category TEXT NOT NULL,
  node_id TEXT NOT NULL,
  PRIMARY KEY (capture_id, category, node_id),
  FOREIGN KEY (capture_id, category) REFERENCES searches(capture_id, category) ON DELETE CASCADE,
  FOREIGN KEY (capture_id, node_id) REFERENCES pull_requests(capture_id, node_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS pull_requests_updated
  ON pull_requests(capture_id, updated_at DESC);
