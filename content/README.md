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
are required.

`development/pack.json` maps the existing fixture to draft objectives,
vocabulary-sense IDs, a unit and explicit exercise skill tags. Run
`npm run check:content` to validate references, compatibility, file integrity and
provenance and report coverage gaps. Its duration is deliberately unmeasured.
Passing preflight cannot approve the fixture or install it in the learner app.

See [the text-pack contract](../docs/CONTENT-PACKS.md). Full reviewed objectives,
media/alternative records and the pack installation/catalogue lifecycle still
need implementation and real editorial work. This is not the completed public
pack format.
