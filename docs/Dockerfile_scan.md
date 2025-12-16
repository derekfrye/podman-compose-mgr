Dockerfile-Based View Mode Plan
===============================

Goal
- Add a fourth TUI view: "List by Dockerfile".
- Discover Dockerfiles (honor path/include/exclude), infer likely image names, and show expansion details similar to "List by image".

High-Level Approach
- Discovery: extend scanning to collect Dockerfiles (e.g., filenames starting with `Dockerfile`), using the same include/exclude/path logic as existing discovery.
- Heuristics for inferred image:
  1) Single-neighbor rule: if a directory has exactly one Dockerfile *and* exactly one quadlet or one compose file, use the first Image=/image: entry from that file.
  2) Registry suffix rule: otherwise, use Dockerfile suffix (e.g., `Dockerfile.xyz` → `xyz`) to match localhost images (repository starts with `localhost/`, tag arbitrary). If multiple matches, pick newest by created date. If none, mark as unknown.
- Podman lookup: reuse/extend podman adapter to fetch local images once per scan (cache for the pass); use created timestamps for tie-break.
- UI: add view picker entry "List by Dockerfile", rows built from Dockerfile findings, expand to show inference source and details.

Data/Code Changes (targeted)
- Domain/discovery:
  - Add discovery for Dockerfiles using existing walker (`walk_dirs` or discovery adapter), respecting include/exclude/path args.
  - Add a Dockerfile-specific result type (path, dir, basename, inferred image info, optional matched image metadata).
  - Reuse existing quadlet/compose parsers to pull first image when the single-neighbor rule applies.
  - Extend podman image listing port/adapters to provide `localhost` images with created timestamps.
- App state/view:
  - Add `ViewMode::ByDockerfile`; wire default selection index in view picker; header/columns for Dockerfile rows.
  - Build rows: for each discovered Dockerfile produce a row with inferred image name (or placeholder) and source dir.
  - Expansion: mirror "by image" expansion but add leading line `Image name: inferred from <quadlet|compose|localhost>` (or `unknown`); include Dockerfile basename, inferred image string, created date if available.
- Input handling: ensure navigation/expansion works the same as other flat views; folder navigation likely unused here (non-hierarchical).

Heuristic Details
- Single-neighbor:
  - Directory must contain exactly one Dockerfile and exactly one eligible quadlet or exactly one docker-compose file (not both >1).
  - Use the first Image=/image: found in that single file.
- Registry suffix:
  - Suffix: `Dockerfile` → empty suffix (likely no match); `Dockerfile.xyz` → `xyz`.
  - Filter podman images where repository starts with `localhost/` and name contains `/suffix` (case-sensitive) or ends with `/suffix`. If multiple, pick newest by created.
  - If no match, display inferred image as unknown.

Performance/Resilience
- Single podman image listing per scan; avoid per-row calls.
- Gracefully handle podman unavailable/empty cache; rows stay but inference source `unknown`.
- Keep existing scan flow unchanged for other modes.

Testing Plan
- Unit-ish fixtures (reuse existing parsing helpers):
  - Single Dockerfile + single quadlet with Image=foo/bar: expect inferred from quadlet → foo/bar.
  - Single Dockerfile + single compose with image: foo/baz: inferred from compose.
  - Multiple Dockerfiles + podman list containing `localhost/djf/xyz` newest: `Dockerfile.xyz` picks localhost/djf/xyz (source: localhost).
  - Multiple matches (localhost/djf/xyz older, localhost/djf2/xyz newer): picks newest.
  - No matches: inferred image unknown, source unknown.
  - Plain `Dockerfile` with suffix empty: falls back to unknown unless neighbor rule applies.
- Integration-style:
  - TUI view picker shows "List by Dockerfile" and selecting it renders rows with basenames.
  - Expansion displays: "Image name: inferred from <source>", Dockerfile basename, inferred image, created date when present.
  - Include/exclude/path respected: set include/exclude args in fixture dirs and verify only expected Dockerfiles appear.
- Podman adapter:
  - Mock `podman image ls` output (via mock_podman) to cover newest-selection and localhost filtering.

Open Questions / Decisions
- Exact match rule for suffix: start-with vs ends-with; proposal above uses ends-with or contains `/suffix` with localhost prefix.
- Formatting of created date: reuse existing image view formatting for consistency.
