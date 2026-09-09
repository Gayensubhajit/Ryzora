# Ryzora Versioning & Schema Evolution Policy

## 1. Application Versioning

Ryzora strictly adheres to [Semantic Versioning 2.0.0](https://semver.org/):
`vMAJOR.MINOR.PATCH`

- **MAJOR**: Incompatible API or database schema migrations, removal of deprecated manifest fields, or breaking protocol alterations.
- **MINOR**: New functional modules, backward-compatible features, desktop environment additions, and packaging improvements.
- **PATCH**: Bug fixes, security hardenings, and dependency updates that do not alter the external API or file layout.

---

## 2. Schema Specifications

To guarantee long-term stability and prevent breaking user setups during upgrades, Ryzora maintains explicit schema versions for all file formats:

### Package Manifest (`ryzora.json` / `manifest.json`)
- **Current Version**: `"ryzora_spec": "1"`
- **Rules**:
  - Unrecognized keys are discarded or flagged as warnings.
  - Required fields: `ryzora_spec`, `id`, `name`, `version`, `author`, `package_type`, `files`.
  - Target paths must begin with `~/` or be home-relative. Absolute root paths (`/etc`, `/usr`) are strictly rejected.

### Collections (`.ryzlist`)
- **Current Version**: `"ryzora_collection": "1"`
- **Rules**:
  - Human-readable JSON format.
  - Describes declarative packages, optional SemVer constraints, and optional repository pinning.
  - Zero executable install instructions or script hooks.

### Settings Configuration (`settings.json`)
- **Current Version**: `"schema_version": 1`
- **Rules**:
  - Persisted with strict Unix mode `0600` in `~/.config/ryzora/settings.json`.
  - Bounded numerical ranges (e.g. refresh intervals 5m–1440m).
  - Malformed or unknown configurations cleanly fallback to secure defaults.

---

## 3. Backward Compatibility & Deprecation

- Old schema versions (e.g. `spec: "1"`) will be supported for at least one full major version cycle after a successor specification is published.
- Automated migrations must never modify user files without taking a pre-migration snapshot.
