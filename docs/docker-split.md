# Split Docker images

The existing service names and runtime contract in `compose.yaml` remain
supported. Compose now consumes the parallel core/desktop images; the original
`Dockerfile` remains available as a fallback. Build the split images with:

```sh
./scripts/build-split-images
```

This produces:

- `review-radar-core:local`: the collector, queue, seed-export, and state
  binaries with only their minimal runtime dependencies.
- `review-radar-desktop:local`: the core image plus Qt/QML and the standalone
  Linux desktop application.

The desktop build consumes the tagged core image through `CORE_IMAGE`; build
the core image first. The existing Compose services, `/data` paths, binary
names, environment variables, and entrypoints are intentionally unchanged.
`scripts/desktop` performs this build automatically before launching Compose.

Smoke-test the split images without changing the current runtime:

```sh
docker run --rm --entrypoint sh review-radar-core:local -ceu '\
  command -v review-radar-github && \
  command -v review-radar-queue && \
  command -v review-radar-state && \
  review-radar-state --database /tmp/state.sqlite3 acknowledge \
    --pull-request-id smoke --fingerprint smoke'
docker run --rm --entrypoint sh review-radar-desktop:local -ceu '\
  test -x /usr/local/bin/review-radar-linux && \
  test -x /usr/local/bin/review-radar-desktop'
```

The desktop image build compiles the production application with
`BUILD_TESTING=OFF`. The existing Qt `--smoke-test` currently requires
follow-up in the active UI tree; the split image does not alter that
application or the legacy desktop image.
