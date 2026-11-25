# GLOSSARY

Canonical terms for the decision record. Implementation-free: no file paths as definitions, no behavior smuggled in.

- **Decision:** A settled choice with a date, rationale and commitment. Identity is the ADR number; the old D-id survives only as an alias during migration.
- **Ruling:** The owner's explicit verdict on a question put with evidence and priced options. A decision without a ruling is unruled, not implied.
- **Conformance:** The verdict on whether the engine honors the decision (e.g. same, deviation, unproven). A claim without a verdict is ungraded.
- **Fixture:** The named test, file or command that pins the decision. A ruling that rests on behavior and names no fixture is **believed implemented**.
- **Unproven:** A migrated row whose evidence section has no rung-0 (game) support. Parked openly, never smoothed over.
- **Parked:** A contradiction or out-of-scope item recorded with its reason, to be settled by its own branch. Not a refusal.
- **Superseded:** An old row replaced by a signed ADR. The old text stays until sign-off, then is deleted per the manifest.
- **Manifest:** The single mapping of every D-id to its ADR successor (or parked/dropped-with-reason). The only trace after deletion.
