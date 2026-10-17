//! Saved review interactions. Content authority comes exclusively from the host.
use crate::{
    backup::hash,
    pack_library::ContentContext,
    review::{select_reviews, Rating, ReviewEvidence, ReviewObservation},
    review_storage::{self, ReviewSource},
    scoring, Answer, Exercise, ExerciseKind, ExerciseView, Feedback, Lesson, Store,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

type Result<T> = std::result::Result<T, String>;
const MAX_SESSION_BYTES: usize = 8 * 1024 * 1024;
const SESSION_SIZE: usize = 10;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn json<T: Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(error)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReviewAttempt {
    pub request_id: Uuid,
    pub answer: Option<Answer>,
    pub correct: bool,
    pub hinted: bool,
    pub revealed: bool,
    pub at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReviewItem {
    pub id: Uuid,
    pub source: ReviewSource,
    pub lesson: Lesson,
    pub draft: Option<Answer>,
    pub hinted: bool,
    pub attempts: Vec<ReviewAttempt>,
    pub observation: Option<ReviewObservation>,
}
impl ReviewItem {
    fn exercise(&self) -> Result<&Exercise> {
        self.lesson
            .exercises
            .iter()
            .find(|e| e.id == self.source.exercise_id)
            .ok_or_else(|| "The saved review exercise is missing.".into())
    }
    fn resolved(&self) -> bool {
        self.attempts
            .last()
            .is_some_and(|a| a.correct || a.revealed)
    }
    fn evidence(&self) -> Result<ReviewEvidence> {
        let last = self
            .attempts
            .last()
            .ok_or("Answer this item before continuing.")?;
        Ok(ReviewEvidence {
            first_correct: self.attempts[0].correct,
            final_correct: last.correct,
            hinted: self.hinted,
            revealed: last.revealed,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SavedReviewSession {
    pub id: Uuid,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub study_date: Option<String>,
    pub position: usize,
    pub items: Vec<ReviewItem>,
    pub xp_earned: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSessionView {
    pub id: String,
    pub status: String,
    pub position: usize,
    pub item_count: usize,
    pub exercise: Option<ExerciseView>,
    pub target: Option<crate::review::ReviewKey>,
    pub can_rate_easy: bool,
    pub xp_earned: u32,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewOverview {
    pub due_count: usize,
    pub available_count: usize,
    pub session_limit: usize,
    pub active: Option<ReviewSessionView>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReviewCredit {
    pub study_date: String,
    pub session_id: Uuid,
    pub amount: u32,
}

impl SavedReviewSession {
    pub(crate) fn view(&self) -> Result<ReviewSessionView> {
        let item = (self.status == "active")
            .then(|| self.items.get(self.position))
            .flatten();
        let exercise = item
            .map(|i| -> Result<ExerciseView> {
                let e = i.exercise()?;
                let last = i.attempts.last();
                let expected = e
                    .accepted
                    .first()
                    .ok_or("Saved review has no answer key.")?
                    .clone();
                Ok(ExerciseView {
                    id: i.id.to_string(),
                    kind: e.kind.clone(),
                    instruction: e.instruction.clone(),
                    prompt: e.prompt.clone(),
                    context: e.context.clone(),
                    options: e.options.clone(),
                    hint: i.hinted.then(|| e.hint.clone()),
                    draft: i.draft.clone(),
                    feedback: last.map(|a| Feedback {
                        correct: a.correct,
                        revealed: a.revealed,
                        expected: expected.clone(),
                        explanation: e.explanation.clone(),
                    }),
                    ready_to_continue: i.resolved(),
                })
            })
            .transpose()?;
        Ok(ReviewSessionView {
            id: self.id.to_string(),
            status: self.status.clone(),
            position: self.position,
            item_count: self.items.len(),
            exercise,
            target: item.map(|i| i.source.key.clone()),
            can_rate_easy: item.is_some_and(|i| {
                i.resolved()
                    && i.evidence()
                        .is_ok_and(|e| e.rating(false) == Ok(Rating::Good))
            }),
            xp_earned: self.xp_earned,
        })
    }
    pub(crate) fn validate(&self, context: &ContentContext, now: DateTime<Utc>) -> Result<()> {
        if self.id.is_nil()
            || self.items.is_empty()
            || self.items.len() > SESSION_SIZE
            || !["active", "completed", "aborted"].contains(&self.status.as_str())
            || self.position > self.items.len()
            || (self.status == "completed") != (self.position == self.items.len())
            || (self.status != "active") != self.ended_at.is_some()
            || self.started_at > now
            || self
                .ended_at
                .is_some_and(|t| t < self.started_at || t > now)
            || (self.status == "completed") != self.study_date.is_some()
            || (self.status != "completed" && self.xp_earned != 0)
            || ![0, 5].contains(&self.xp_earned)
        {
            return Err("Invalid saved review session.".into());
        }
        let mut keys = HashSet::new();
        let mut ids = HashSet::new();
        let mut requests = HashSet::new();
        let mut previous = self.started_at;
        for (position, i) in self.items.iter().enumerate() {
            crate::store::validate_lesson(&i.lesson)?;
            let e = i.exercise()?;
            if i.id.is_nil()
                || !ids.insert(i.id)
                || !keys.insert(&i.source.key)
                || !context.review_sources.contains(&i.source)
                || i.lesson.editorial_status != "approved"
                || i.source.lesson_id != i.lesson.id
                || i.source.lesson_sha256 != hash(&i.lesson)?
                || i.attempts.len() > 10_000
                || (position > self.position
                    && (i.hinted || i.draft.is_some() || !i.attempts.is_empty()))
            {
                return Err(
                    "Review source or saved interaction is invalid; repair its verified pack."
                        .into(),
                );
            }
            if let Some(draft) = &i.draft {
                validate_draft(e, draft)?;
            }
            let mut resolved = false;
            let mut hinted = false;
            for a in &i.attempts {
                if a.request_id.is_nil()
                    || !requests.insert(a.request_id)
                    || resolved
                    || a.at < previous
                    || a.at > now
                    || self.ended_at.is_some_and(|end| a.at > end)
                    || (hinted && !a.hinted)
                    || (a.hinted && !i.hinted)
                    || (a.revealed != a.answer.is_none())
                    || a.correct
                        != a.answer
                            .as_ref()
                            .map(|v| scoring::evaluate(e, v))
                            .transpose()?
                            .unwrap_or(false)
                {
                    return Err("Saved review answer evidence is inconsistent.".into());
                }
                resolved = a.correct || a.revealed;
                hinted = a.hinted;
                previous = a.at;
            }
            if i.attempts
                .last()
                .is_some_and(|a| !a.revealed && i.draft.as_ref() != a.answer.as_ref())
                && resolved
            {
                return Err("Resolved review draft differs from its submitted answer.".into());
            }
            if i.observation.is_some() != (position < self.position) {
                return Err("Review position and ratings do not agree.".into());
            }
            if let Some(o) = &i.observation {
                if !resolved
                    || o.session_id != self.id
                    || o.request_id != i.attempts.last().unwrap().request_id
                    || o.evidence != i.evidence()?
                    || o.reviewed_at < previous
                    || o.reviewed_at > now
                    || self.ended_at.is_some_and(|end| o.reviewed_at > end)
                {
                    return Err("Review rating does not match its saved answer.".into());
                }
                o.evidence.rating(o.explicit_easy)?;
                previous = o.reviewed_at;
            }
        }
        Ok(())
    }
}
fn validate_draft(e: &Exercise, a: &Answer) -> Result<()> {
    match (&e.kind, a) {
        (ExerciseKind::Translation | ExerciseKind::Cloze, Answer::Text(s)) if s.len() <= 4000 => {
            Ok(())
        }
        (ExerciseKind::Ordering, Answer::Tiles(ids)) if ids.is_empty() => Ok(()),
        _ => scoring::answer_text(e, a).map(|_| ()),
    }
}
pub(crate) fn save(connection: &Connection, s: &SavedReviewSession) -> Result<()> {
    let data = json(s)?;
    if data.len() > MAX_SESSION_BYTES {
        return Err("Saved review exceeds the supported size limit.".into());
    }
    connection.execute("INSERT INTO review_sessions(id,status,record) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET status=excluded.status,record=excluded.record",params![s.id.to_string(),s.status,data]).map_err(error)?;
    Ok(())
}
fn load(connection: &Connection, id: &str) -> Result<SavedReviewSession> {
    let (status, length): (String, i64) = connection
        .query_row(
            "SELECT status,length(CAST(record AS BLOB)) FROM review_sessions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(error)?;
    if length > MAX_SESSION_BYTES as i64 {
        return Err("Saved review exceeds the supported size limit.".into());
    }
    let data: String = connection
        .query_row(
            "SELECT record FROM review_sessions WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(error)?;
    let s: SavedReviewSession = serde_json::from_str(&data).map_err(error)?;
    if s.id.to_string() != id || s.status != status {
        return Err("Saved review index does not match its contents.".into());
    }
    Ok(s)
}
pub(crate) fn read_all(connection: &Connection) -> Result<Vec<SavedReviewSession>> {
    if review_storage::schema_version(connection)? < 3 {
        return Ok(vec![]);
    }
    let (count, total): (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(length(CAST(record AS BLOB))),0) FROM review_sessions",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(error)?;
    if count > 100_000 || total > crate::MAX_BACKUP_BYTES as i64 {
        return Err("Saved reviews exceed supported backup limits.".into());
    }
    let mut statement = connection
        .prepare("SELECT id FROM review_sessions ORDER BY id")
        .map_err(error)?;
    let ids = statement
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    ids.iter().map(|id| load(connection, id)).collect()
}
pub(crate) fn credits(connection: &Connection) -> Result<Vec<ReviewCredit>> {
    if review_storage::schema_version(connection)? < 3 {
        return Ok(vec![]);
    }
    let mut statement = connection
        .prepare("SELECT study_date,session_id,amount FROM review_xp ORDER BY study_date")
        .map_err(error)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })
        .map_err(error)?;
    rows.map(|r| {
        let (study_date, id, amount) = r.map_err(error)?;
        Ok(ReviewCredit {
            study_date,
            session_id: Uuid::parse_str(&id).map_err(error)?,
            amount,
        })
    })
    .collect()
}

impl Store {
    fn review_candidates(
        &self,
        now: DateTime<Utc>,
        practice: bool,
    ) -> Result<Vec<(ReviewSource, Lesson)>> {
        let mut choices = BTreeMap::new();
        for lesson in &self.lessons {
            let digest = hash(lesson)?;
            let mut sources: Vec<_> = self
                .content
                .review_sources
                .iter()
                .filter(|s| s.lesson_sha256 == digest)
                .collect();
            sources.sort_by_key(|s| (&s.pack.pack_id, s.pack.version, &s.exercise_id));
            for source in sources {
                choices
                    .entry(source.key.clone())
                    .or_insert((source.clone(), lesson.clone()));
            }
        }
        // Only items already introduced through graded lessons are reviewable.
        let mut candidates = vec![];
        for record in self.review_histories()? {
            record.validate_authority(&self.content)?;
            if choices.contains_key(record.history().key()) {
                candidates.push(record.history().candidate()?);
            }
        }
        select_reviews(&candidates, now, 10_000, practice)?
            .into_iter()
            .map(|c| {
                choices
                    .remove(&c.key)
                    .ok_or_else(|| "Review content unavailable.".into())
            })
            .collect()
    }
    pub fn review_overview(&self, now: DateTime<Utc>) -> Result<ReviewOverview> {
        let active: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM review_sessions WHERE status='active'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        let active = active.map(|id| self.review_session(&id)).transpose()?;
        // A broken/revoked pack must not hide the action to end a saved review.
        let (due_count, available_count) = if active.is_some() {
            (0, 0)
        } else {
            (
                self.review_candidates(now, false)?.len(),
                self.review_candidates(now, true)?.len(),
            )
        };
        Ok(ReviewOverview {
            due_count,
            available_count,
            session_limit: SESSION_SIZE,
            active,
        })
    }
    pub fn review_session(&self, id: &str) -> Result<ReviewSessionView> {
        load(&self.connection, id)?.view()
    }
    pub fn start_review(
        &mut self,
        practice: bool,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        self.settings()?
            .ok_or("Choose your study settings first.")?;
        let choices = self.review_candidates(now, practice)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let active: Option<String> = tx
            .query_row(
                "SELECT id FROM review_sessions WHERE status='active'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        if let Some(id) = active {
            let s = load(&tx, &id)?;
            s.validate(&self.content, now)?;
            return s.view();
        }
        let lesson: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE status='active')",
                [],
                |r| r.get(0),
            )
            .map_err(error)?;
        if lesson {
            return Err("Finish the saved lesson before starting a review.".into());
        }
        let items = choices
            .into_iter()
            .take(SESSION_SIZE)
            .map(|(source, lesson)| ReviewItem {
                id: Uuid::new_v4(),
                source,
                lesson,
                draft: None,
                hinted: false,
                attempts: vec![],
                observation: None,
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err("No eligible reviewed items are available. Complete a lesson from an independently reviewed pack first.".into());
        }
        let s = SavedReviewSession {
            id: Uuid::new_v4(),
            status: "active".into(),
            started_at: now,
            ended_at: None,
            study_date: None,
            position: 0,
            items,
            xp_earned: 0,
        };
        save(&tx, &s)?;
        tx.commit().map_err(error)?;
        s.view()
    }
    fn edit_review(
        &mut self,
        id: &str,
        item_id: &str,
        now: DateTime<Utc>,
        action: impl FnOnce(&Connection, &mut ReviewItem) -> Result<()>,
    ) -> Result<ReviewSessionView> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let mut s = load(&tx, id)?;
        s.validate(&self.content, now)?;
        if s.status != "active" || s.items[s.position].id.to_string() != item_id {
            return Err("This review item is no longer active. Resume the saved review.".into());
        }
        action(&tx, &mut s.items[s.position])?;
        s.validate(&self.content, now)?;
        save(&tx, &s)?;
        tx.commit().map_err(error)?;
        s.view()
    }
    pub fn review_draft(
        &mut self,
        id: &str,
        item: &str,
        answer: Answer,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        self.edit_review(id, item, now, |_, i| {
            if !i.resolved() {
                validate_draft(i.exercise()?, &answer)?;
                i.draft = Some(answer);
            }
            Ok(())
        })
    }
    pub fn review_hint(
        &mut self,
        id: &str,
        item: &str,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        self.edit_review(id, item, now, |_, i| {
            if !i.resolved() {
                i.hinted = true;
            }
            Ok(())
        })
    }
    pub fn review_submit(
        &mut self,
        id: &str,
        item: &str,
        answer: Answer,
        request: &str,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        let request = Uuid::parse_str(request).map_err(error)?;
        if request.is_nil() {
            return Err("Invalid submission identifier.".into());
        }
        let s = load(&self.connection, id)?;
        if let Some((i, a)) = s.items.iter().find_map(|i| {
            i.attempts
                .iter()
                .find(|a| a.request_id == request)
                .map(|a| (i, a))
        }) {
            if i.id.to_string() != item || a.answer.as_ref() != Some(&answer) {
                return Err("Submission identifier was reused with different review data.".into());
            }
            return s.view();
        }
        self.edit_review(id, item, now, |connection, i| {
        let used:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM attempts WHERE request_id=?1) OR EXISTS(SELECT 1 FROM review_sessions r, json_each(r.record,'$.items') i, json_each(i.value,'$.attempts') a WHERE json_extract(a.value,'$.requestId')=?1)",[request.to_string()],|r|r.get(0)).map_err(error)?;
        if used {
            return Err("Submission identifier was already used for another interaction.".into());
        }
            if i.resolved() {
                return Err("Continue after reviewing this answer.".into());
            }
            let correct = scoring::evaluate(i.exercise()?, &answer)?;
            i.draft = Some(answer.clone());
            i.attempts.push(ReviewAttempt {
                request_id: request,
                answer: Some(answer),
                correct,
                hinted: i.hinted,
                revealed: false,
                at: now,
            });
            Ok(())
        })
    }
    pub fn review_reveal(
        &mut self,
        id: &str,
        item: &str,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        self.edit_review(id, item, now, |_, i| {
            if !i.resolved() {
                i.attempts.push(ReviewAttempt {
                    request_id: Uuid::new_v4(),
                    answer: None,
                    correct: false,
                    hinted: i.hinted,
                    revealed: true,
                    at: now,
                });
            }
            Ok(())
        })
    }
    pub fn abort_review(&mut self, id: &str, now: DateTime<Utc>) -> Result<ReviewSessionView> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let mut s = load(&tx, id)?;
        // Abandoning unavailable content must remain possible; it earns no credit.
        if s.status == "active" {
            if now < s.started_at
                || s.items.iter().any(|i| {
                    i.attempts.iter().any(|a| a.at > now)
                        || i.observation.as_ref().is_some_and(|o| o.reviewed_at > now)
                })
            {
                return Err("Review time precedes saved interactions.".into());
            }
            s.status = "aborted".into();
            s.ended_at = Some(now);
            save(&tx, &s)?;
        }
        tx.commit().map_err(error)?;
        s.view()
    }
    pub fn advance_review(
        &mut self,
        id: &str,
        item: &str,
        easy: bool,
        now: DateTime<Utc>,
    ) -> Result<ReviewSessionView> {
        let settings = self.settings()?.ok_or("Study settings are missing.")?;
        let date = crate::store::study_date(&settings, now)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let mut s = load(&tx, id)?;
        s.validate(&self.content, now)?;
        if s.status != "active" || s.items[s.position].id.to_string() != item {
            return s.view();
        }
        let i = &mut s.items[s.position];
        if !i.resolved() {
            return Err("Correct the answer or review the solution before continuing.".into());
        }
        let observation = ReviewObservation {
            request_id: i.attempts.last().unwrap().request_id,
            session_id: s.id,
            reviewed_at: now,
            evidence: i.evidence()?,
            explicit_easy: easy,
        };
        review_storage::record_review(&tx, &self.content, &i.source, observation.clone())?;
        i.observation = Some(observation);
        s.position += 1;
        if s.position == s.items.len() {
            s.status = "completed".into();
            s.ended_at = Some(now);
            s.study_date = Some(date.clone());
            // Save the parent row before the foreign-key protected credit.
            save(&tx, &s)?;
            tx.execute(
                "INSERT OR IGNORE INTO study_days(date,time_zone,goal) VALUES(?1,?2,?3)",
                params![date, settings.time_zone, settings.daily_goal],
            )
            .map_err(error)?;
            let inserted=tx.execute("INSERT INTO review_xp(study_date,session_id,amount) VALUES(?1,?2,5) ON CONFLICT(study_date) DO NOTHING",params![date,id]).map_err(error)?;
            s.xp_earned = if inserted == 1 { 5 } else { 0 };
            crate::store::grant_achievements(&tx, "first_review", &date, now)?;
        }
        s.validate(&self.content, now)?;
        save(&tx, &s)?;
        tx.commit().map_err(error)?;
        s.view()
    }
}
