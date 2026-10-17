use crate::{model::*, scoring, LEVELS};
use chrono::{DateTime, Days, NaiveDate, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};
use std::{collections::HashSet, path::Path};
use uuid::Uuid;

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(error)
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(error)
}

pub struct Store {
    pub(crate) connection: Connection,
    pub(crate) lessons: Vec<Lesson>,
    pub(crate) development_build: bool,
    pub(crate) content: crate::pack_library::ContentContext,
    pub(crate) content_issues: Vec<String>,
}

impl Store {
    pub fn open(path: &Path, lessons: Vec<Lesson>, development_build: bool) -> Result<Self> {
        let connection = Connection::open(path).map_err(error)?;
        Self::initialize(connection, lessons, development_build)
    }

    #[cfg(test)]
    pub(crate) fn memory(lessons: Vec<Lesson>) -> Result<Self> {
        Self::initialize(Connection::open_in_memory().map_err(error)?, lessons, true)
    }

    pub(crate) fn initialize(
        mut connection: Connection,
        lessons: Vec<Lesson>,
        development_build: bool,
    ) -> Result<Self> {
        for lesson in &lessons {
            validate_lesson(lesson)?;
        }
        if lessons.iter().map(|l| &l.id).collect::<HashSet<_>>().len() != lessons.len() {
            return Err("Duplicate lesson identifiers.".into());
        }
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
            .map_err(error)?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(error)?;
        match version {
            0 => {
                // Fresh storage has no learner data to back up. Existing version-1
                // stores require the explicit, verified recovery/migration path.
                let tx = connection.transaction().map_err(error)?;
                tx.execute_batch(include_str!("schema.sql")).map_err(error)?;
                tx.execute_batch(include_str!("review_schema.sql")).map_err(error)?;
                tx.commit().map_err(error)?;
            }
            1 | 2 => {}
            _ => return Err(format!("Database schema {version} is newer than this application supports. Your data was not changed.")),
        }
        Ok(Self {
            connection,
            lessons,
            development_build,
            content: Default::default(),
            content_issues: vec![],
        })
    }

    pub fn settings(&self) -> Result<Option<Settings>> {
        let value: Option<String> = self
            .connection
            .query_row("SELECT settings FROM profile WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(error)?;
        value.map(|value| decode(&value)).transpose()
    }

    pub fn save_settings(&mut self, settings: Settings, now: DateTime<Utc>) -> Result<AppState> {
        validate_settings(&settings)?;
        let today = study_date(&settings, now)?;
        let tx = self.connection.transaction().map_err(error)?;
        tx.execute("INSERT INTO profile(id, settings) VALUES(1, ?1) ON CONFLICT(id) DO UPDATE SET settings=excluded.settings", [json(&settings)?]).map_err(error)?;
        tx.execute(
            "UPDATE study_days SET goal=?1 WHERE date=?2",
            params![settings.daily_goal, today],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        self.state(now)
    }

    pub fn state(&self, now: DateTime<Utc>) -> Result<AppState> {
        let active: Option<String> = self
            .connection
            .query_row("SELECT id FROM sessions WHERE status='active'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(error)?;
        Ok(AppState {
            settings: self.settings()?,
            lessons: self.lessons.iter().map(Lesson::summary).collect(),
            active_session: active.map(|id| self.session(&id)).transpose()?,
            progress: self.progress(now)?,
            development_build: self.development_build,
            content_issues: self.content_issues.clone(),
        })
    }

    pub fn start(&mut self, lesson_id: &str, now: DateTime<Utc>) -> Result<SessionView> {
        if self.settings()?.is_none() {
            return Err("Choose your study settings first.".into());
        }
        let active: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT id,lesson_id FROM sessions WHERE status='active'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(error)?;
        if let Some((id, current_lesson)) = active {
            if current_lesson == lesson_id {
                return self.session(&id);
            }
            return Err("Resume the saved lesson before starting another one.".into());
        }
        let lesson = self
            .lessons
            .iter()
            .find(|l| l.id == lesson_id)
            .ok_or("This lesson is not installed.")?;
        let practice = lesson.editorial_status != "approved";
        self.check_snapshot_authority(lesson)?;
        if practice && !self.development_build {
            return Err("Unreviewed lessons are unavailable in this build.".into());
        }
        let id = Uuid::new_v4().to_string();
        let tx = self.connection.transaction().map_err(error)?;
        tx.execute("INSERT INTO sessions(id,lesson_id,lesson_version,lesson_snapshot,status,practice_only,started_at) VALUES(?1,?2,?3,?4,'active',?5,?6)", params![id,lesson.id,lesson.version,json(lesson)?,practice,now.to_rfc3339()]).map_err(error)?;
        for position in 0..lesson.exercises.len() {
            tx.execute(
                "INSERT INTO session_items(session_id,position) VALUES(?1,?2)",
                params![id, position as u32],
            )
            .map_err(error)?;
        }
        tx.commit().map_err(error)?;
        self.session(&id)
    }

    fn snapshot(&self, id: &str) -> Result<(Lesson, usize, String, bool, u32)> {
        let (snapshot, position, status, practice, xp): (String, usize, String, bool, u32) = self.connection.query_row("SELECT lesson_snapshot,position,status,practice_only,xp_earned FROM sessions WHERE id=?1", [id], |r| Ok((r.get(0)?,r.get::<_,u32>(1)? as usize,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(error)?;
        Ok((decode(&snapshot)?, position, status, practice, xp))
    }

    pub fn session(&self, id: &str) -> Result<SessionView> {
        let (lesson, position, status, practice_only, xp_earned) = self.snapshot(id)?;
        let (first_correct, independent_correct, hinted_items, revealed_items) = self.connection.query_row("SELECT COALESCE(SUM(first_correct),0),COALESCE(SUM(first_independent),0),COALESCE(SUM(hinted),0),COALESCE(SUM(revealed),0) FROM session_items WHERE session_id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(error)?;
        let exercise = if status == "active" {
            let current = lesson
                .exercises
                .get(position)
                .ok_or("Saved lesson position is invalid.")?;
            let (draft,hinted,resolved,feedback): (Option<String>,bool,bool,Option<String>) = self.connection.query_row("SELECT draft,hinted,resolved,feedback FROM session_items WHERE session_id=?1 AND position=?2", params![id,position as u32], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(error)?;
            Some(ExerciseView {
                id: current.id.clone(),
                kind: current.kind.clone(),
                instruction: current.instruction.clone(),
                prompt: current.prompt.clone(),
                context: current.context.clone(),
                options: current.options.clone(),
                hint: hinted.then(|| current.hint.clone()),
                draft: draft.map(|value| decode(&value)).transpose()?,
                feedback: feedback.map(|value| decode(&value)).transpose()?,
                ready_to_continue: resolved,
            })
        } else {
            None
        };
        Ok(SessionView {
            id: id.into(),
            lesson: lesson.summary(),
            position,
            status,
            exercise,
            first_correct,
            independent_correct,
            hinted_items,
            revealed_items,
            xp_earned,
            practice_only,
        })
    }

    fn check_snapshot_authority(&self, lesson: &Lesson) -> Result<()> {
        if self.content.managed
            && lesson.editorial_status == "approved"
            && !self
                .content
                .reviewed_snapshots
                .contains(&crate::backup::hash(lesson)?)
        {
            return Err("This reviewed lesson needs its matching verified pack. Open Settings to repair or reinstall it; saved progress has been kept.".into());
        }
        Ok(())
    }

    fn active_item(&self, id: &str, exercise_id: &str) -> Result<(Lesson, usize)> {
        let (lesson, position, status, _, _) = self.snapshot(id)?;
        self.check_snapshot_authority(&lesson)?;
        if status != "active"
            || lesson.exercises.get(position).map(|e| e.id.as_str()) != Some(exercise_id)
        {
            return Err("This exercise is no longer active. Reload the saved session.".into());
        }
        Ok((lesson, position))
    }

    pub fn save_draft(&mut self, id: &str, exercise_id: &str, answer: Answer) -> Result<()> {
        let (lesson, position) = self.active_item(id, exercise_id)?;
        // Incomplete text/empty tile drafts are allowed without scoring them.
        match (&lesson.exercises[position].kind, &answer) {
            (ExerciseKind::Translation | ExerciseKind::Cloze, Answer::Text(value))
                if value.len() <= 4000 => {}
            (ExerciseKind::Ordering, Answer::Tiles(ids)) if ids.is_empty() => {}
            _ => {
                scoring::answer_text(&lesson.exercises[position], &answer)?;
            }
        }
        self.connection.execute("UPDATE session_items SET draft=?1 WHERE session_id=?2 AND position=?3 AND resolved=0", params![json(&answer)?,id,position as u32]).map_err(error)?;
        Ok(())
    }

    pub fn hint(&mut self, id: &str, exercise_id: &str) -> Result<SessionView> {
        let (_, position) = self.active_item(id, exercise_id)?;
        self.connection.execute("UPDATE session_items SET hinted=1 WHERE session_id=?1 AND position=?2 AND resolved=0", params![id,position as u32]).map_err(error)?;
        self.session(id)
    }

    pub fn submit(
        &mut self,
        id: &str,
        exercise_id: &str,
        answer: Answer,
        request_id: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionView> {
        Uuid::parse_str(request_id).map_err(|_| "Invalid submission identifier.")?;
        let previous: Option<(String, usize)> = self
            .connection
            .query_row(
                "SELECT session_id,position FROM attempts WHERE request_id=?1",
                [request_id],
                |r| Ok((r.get(0)?, r.get::<_, u32>(1)? as usize)),
            )
            .optional()
            .map_err(error)?;
        if let Some((previous_id, previous_position)) = previous {
            let (lesson, _, _, _, _) = self.snapshot(id)?;
            if previous_id != id
                || lesson
                    .exercises
                    .get(previous_position)
                    .map(|e| e.id.as_str())
                    != Some(exercise_id)
            {
                return Err("Submission identifier was already used for another exercise.".into());
            }
            return self.session(id);
        }
        let (lesson, position) = self.active_item(id, exercise_id)?;
        let exercise = &lesson.exercises[position];
        let correct = scoring::evaluate(exercise, &answer)?;
        let feedback = Feedback {
            correct,
            revealed: false,
            expected: exercise.accepted[0].clone(),
            explanation: exercise.explanation.clone(),
        };
        let tx = self.connection.transaction().map_err(error)?;
        let (hinted, revealed, resolved): (bool,bool,bool) = tx.query_row("SELECT hinted,revealed,resolved FROM session_items WHERE session_id=?1 AND position=?2", params![id,position as u32], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(error)?;
        if resolved {
            return Err("This item is already resolved. Continue to the next exercise.".into());
        }
        tx.execute("INSERT INTO attempts(request_id,session_id,position,correct,hinted,revealed,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![request_id,id,position as u32,correct,hinted,revealed,now.to_rfc3339()]).map_err(error)?;
        tx.execute("UPDATE session_items SET first_correct=COALESCE(first_correct,?1),first_independent=COALESCE(first_independent,?2),resolved=?1,feedback=?3,draft=?4 WHERE session_id=?5 AND position=?6", params![correct,correct && !hinted && !revealed,json(&feedback)?,json(&answer)?,id,position as u32]).map_err(error)?;
        tx.commit().map_err(error)?;
        self.session(id)
    }

    pub fn reveal(
        &mut self,
        id: &str,
        exercise_id: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionView> {
        let (lesson, position) = self.active_item(id, exercise_id)?;
        let exercise = &lesson.exercises[position];
        let feedback = Feedback {
            correct: false,
            revealed: true,
            expected: exercise.accepted[0].clone(),
            explanation: exercise.explanation.clone(),
        };
        let tx = self.connection.transaction().map_err(error)?;
        let resolved: bool = tx
            .query_row(
                "SELECT resolved FROM session_items WHERE session_id=?1 AND position=?2",
                params![id, position as u32],
                |r| r.get(0),
            )
            .map_err(error)?;
        if !resolved {
            tx.execute("INSERT INTO attempts(request_id,session_id,position,correct,hinted,revealed,created_at) SELECT ?1,?2,?3,0,hinted,1,?4 FROM session_items WHERE session_id=?2 AND position=?3", params![Uuid::new_v4().to_string(),id,position as u32,now.to_rfc3339()]).map_err(error)?;
            tx.execute("UPDATE session_items SET revealed=1,resolved=1,first_correct=COALESCE(first_correct,0),first_independent=COALESCE(first_independent,0),feedback=?1 WHERE session_id=?2 AND position=?3", params![json(&feedback)?,id,position as u32]).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        self.session(id)
    }

    pub fn advance(
        &mut self,
        id: &str,
        exercise_id: &str,
        now: DateTime<Utc>,
    ) -> Result<SessionView> {
        let (lesson, position, status, practice, _) = self.snapshot(id)?;
        if status == "completed" {
            return self.session(id);
        }
        self.check_snapshot_authority(&lesson)?;
        if lesson.exercises.get(position).map(|e| e.id.as_str()) != Some(exercise_id) {
            // Duplicate Continue requests must never skip an exercise.
            return self.session(id);
        }
        let settings = self.settings()?.ok_or("Study settings are missing.")?;
        let date = study_date(&settings, now)?;
        let tx = self.connection.transaction().map_err(error)?;
        let resolved: bool = tx
            .query_row(
                "SELECT resolved FROM session_items WHERE session_id=?1 AND position=?2",
                params![id, position as u32],
                |r| r.get(0),
            )
            .map_err(error)?;
        if !resolved {
            return Err("Correct the answer or review the solution before continuing.".into());
        }
        if !practice {
            crate::review_storage::seed_lesson_reviews(
                &tx,
                &self.content,
                &lesson,
                id,
                position,
                now,
            )?;
        }
        if position + 1 < lesson.exercises.len() {
            tx.execute("UPDATE sessions SET position=position+1 WHERE id=?1", [id])
                .map_err(error)?;
        } else {
            let mut xp = 0;
            if !practice {
                tx.execute(
                    "INSERT OR IGNORE INTO study_days(date,time_zone,goal) VALUES(?1,?2,?3)",
                    params![date, settings.time_zone, settings.daily_goal],
                )
                .map_err(error)?;
                let inserted = tx.execute("INSERT INTO xp_ledger(id,session_id,lesson_id,study_date,amount) VALUES(?1,?2,?3,?4,10) ON CONFLICT(lesson_id,study_date) DO NOTHING", params![Uuid::new_v4().to_string(),id,lesson.id,date]).map_err(error)?;
                if inserted == 1 {
                    xp = 10;
                }
                tx.execute(
                    "INSERT OR IGNORE INTO achievements(id,granted_at) VALUES('first_lesson',?1)",
                    [now.to_rfc3339()],
                )
                .map_err(error)?;
                let total: u32 = tx
                    .query_row("SELECT COALESCE(SUM(amount),0) FROM xp_ledger", [], |r| {
                        r.get(0)
                    })
                    .map_err(error)?;
                if total >= 100 {
                    tx.execute(
                        "INSERT OR IGNORE INTO achievements(id,granted_at) VALUES('100_xp',?1)",
                        [now.to_rfc3339()],
                    )
                    .map_err(error)?;
                }
                let dates = day_dates(&tx)?;
                if streaks(&dates, &date)?.1 >= 7 {
                    tx.execute("INSERT OR IGNORE INTO achievements(id,granted_at) VALUES('7_day_streak',?1)", [now.to_rfc3339()]).map_err(error)?;
                }
            }
            tx.execute(
                "UPDATE sessions SET status='completed',completed_at=?1,xp_earned=?2 WHERE id=?3",
                params![now.to_rfc3339(), xp, id],
            )
            .map_err(error)?;
        }
        tx.commit().map_err(error)?;
        self.session(id)
    }

    pub fn report_answer(
        &mut self,
        id: &str,
        exercise_id: &str,
        note: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let (lesson, _, _, _, _) = self.snapshot(id)?;
        if !lesson.exercises.iter().any(|e| e.id == exercise_id)
            || note.trim().is_empty()
            || note.len() > 4000
        {
            return Err("Choose an exercise and enter a report of at most 4,000 bytes.".into());
        }
        self.connection.execute("INSERT INTO answer_reports(id,session_id,exercise_id,note,created_at) VALUES(?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(),id,exercise_id,note,now.to_rfc3339()]).map_err(error)?;
        Ok(())
    }

    pub fn progress(&self, now: DateTime<Utc>) -> Result<Progress> {
        let today = if let Some(settings) = self.settings()? {
            study_date(&settings, now)?
        } else {
            now.date_naive().to_string()
        };
        let total_xp = self
            .connection
            .query_row("SELECT COALESCE(SUM(amount),0) FROM xp_ledger", [], |r| {
                r.get(0)
            })
            .map_err(error)?;
        let today_xp = self
            .connection
            .query_row(
                "SELECT COALESCE(SUM(amount),0) FROM xp_ledger WHERE study_date=?1",
                [&today],
                |r| r.get(0),
            )
            .map_err(error)?;
        let completed_lessons = self.connection.query_row("SELECT COUNT(DISTINCT lesson_id) FROM sessions WHERE status='completed' AND practice_only=0", [], |r| r.get(0)).map_err(error)?;
        let mut statement = self.connection.prepare("SELECT d.date,d.goal,d.time_zone,COALESCE(SUM(x.amount),0) FROM study_days d LEFT JOIN xp_ledger x ON x.study_date=d.date GROUP BY d.date ORDER BY d.date").map_err(error)?;
        let days = statement
            .query_map([], |r| {
                Ok(StudyDay {
                    date: r.get(0)?,
                    goal: r.get(1)?,
                    time_zone: r.get(2)?,
                    xp: r.get(3)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let (current_streak, best_streak) = streaks(
            &days.iter().map(|d| d.date.clone()).collect::<Vec<_>>(),
            &today,
        )?;
        let mut statement = self
            .connection
            .prepare("SELECT id FROM achievements ORDER BY id")
            .map_err(error)?;
        let achievements = statement
            .query_map([], |r| r.get(0))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        Ok(Progress {
            total_xp,
            today_xp,
            completed_lessons,
            current_streak,
            best_streak,
            achievements,
            days,
        })
    }
}

pub(crate) fn validate_settings(settings: &Settings) -> Result<()> {
    if !LEVELS.contains(&settings.level.as_str()) {
        return Err("Choose Beginner or a CEFR level from A1 through C2.".into());
    }
    settings
        .time_zone
        .parse::<Tz>()
        .map_err(|_| "Choose a valid IANA study time zone, for example America/Chicago.")?;
    if settings.daily_goal == 0 {
        return Err("The daily goal must be positive.".into());
    }
    if !["system", "light", "dark"].contains(&settings.theme.as_str()) {
        return Err("Unknown theme.".into());
    }
    if !(100..=200).contains(&settings.text_scale) {
        return Err("Text size must be between 100% and 200%.".into());
    }
    Ok(())
}

pub(crate) fn validate_lesson(lesson: &Lesson) -> Result<()> {
    if lesson.id.is_empty()
        || lesson.version == 0
        || lesson.exercises.is_empty()
        || lesson.license.is_empty()
        || !LEVELS.contains(&lesson.level.as_str())
        || !["draft", "approved"].contains(&lesson.editorial_status.as_str())
    {
        return Err("Invalid lesson metadata.".into());
    }
    let mut ids = HashSet::new();
    for exercise in &lesson.exercises {
        if !ids.insert(&exercise.id)
            || exercise.id.is_empty()
            || exercise.instruction.is_empty()
            || exercise.prompt.is_empty()
            || exercise.accepted.is_empty()
            || exercise
                .accepted
                .iter()
                .any(|a| scoring::normalize(a).is_empty())
            || exercise.explanation.is_empty()
        {
            return Err("Invalid or duplicate exercise record.".into());
        }
        if exercise
            .options
            .iter()
            .map(|o| &o.id)
            .collect::<HashSet<_>>()
            .len()
            != exercise.options.len()
        {
            return Err("Duplicate option/tile identifiers.".into());
        }
        if matches!(exercise.kind, ExerciseKind::Choice | ExerciseKind::Reading)
            && !exercise.accepted.iter().all(|a| {
                exercise
                    .options
                    .iter()
                    .any(|o| scoring::normalize(&o.text) == scoring::normalize(a))
            })
        {
            return Err("A choice answer must reference an available option.".into());
        }
    }
    Ok(())
}

fn study_date(settings: &Settings, now: DateTime<Utc>) -> Result<String> {
    let zone = settings.time_zone.parse::<Tz>().map_err(error)?;
    Ok(now.with_timezone(&zone).date_naive().to_string())
}
fn day_dates(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT date FROM study_days ORDER BY date")
        .map_err(error)?;
    let dates = statement
        .query_map([], |r| r.get(0))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(dates)
}
fn streaks(dates: &[String], today: &str) -> Result<(u32, u32)> {
    let today = NaiveDate::parse_from_str(today, "%Y-%m-%d").map_err(error)?;
    let mut previous: Option<NaiveDate> = None;
    let (mut run, mut best, mut current) = (0, 0, 0);
    for value in dates {
        let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(error)?;
        run = if previous.and_then(|d| d.checked_add_days(Days::new(1))) == Some(date) {
            run + 1
        } else {
            1
        };
        best = best.max(run);
        // A time-zone change can leave a historical day ahead of today's local
        // calendar date. Preserve it without hiding an actual current streak.
        if date == today || Some(date) == today.checked_sub_days(Days::new(1)) {
            current = run;
        }
        previous = Some(date);
    }
    Ok((current, best))
}
