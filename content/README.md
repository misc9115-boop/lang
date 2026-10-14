# Content status

No approved curriculum or media packs exist in this repository yet.

`development/cafe.json` is an original AI-drafted **test fixture**, inspired by
the illustrative café example in SPEC.md §5. It has not received linguistic,
pedagogical, answer-variant, or pacing review. Its five items are deliberately
for exercising controls, not a claim to be a complete 5–10 minute lesson.

The fixture is available only in debug/test builds. It cannot earn XP, study-day
credit, achievements, course completion, or mastery. Release builds exclude the
embedded fixture and have an honest empty course library. Never change its
editorial status to `approved` merely to enable progress in the app. A test-only
copy marked approved inside Rust tests exercises awarding rules; it is not an
editorial sign-off.

Original fixture text is offered under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
Attribution: “lang contributors, development café fixture, version 1; original
AI-drafted material; unreviewed.” Third-party content/media retains its own rights.

The current JSON schema is an internal foundation format (Rust `Lesson` and
`Exercise`, with unknown fields rejected). Stable IDs, version, level, title,
objective, editorial status, license, accepted answers, hints and explanations
are required. The content-pack schema still needs the full objective/skill map,
asset manifest, compatibility/integrity rules, review provenance and pack lifecycle
specified by SPEC.md. This fixture format is not a public pack format.
