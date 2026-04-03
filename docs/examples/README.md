This directory is for user-facing example assets that the public docs can
reference directly.

Current contents:

- `dashboard-governance-policy.json`: sample governance policy used by the
  user guides.

These files are documentation samples, not runtime-owned assets. Keep code-owned
copies under the relevant runtime tree when Rust or Python needs a built-in
sample or fixture.

Do not place unwired prototype or maintainer-only demo scripts here. Put those
under `docs/internal/` so `docs/examples/` stays operator-facing.
