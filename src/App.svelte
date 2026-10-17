<script lang="ts">
  import { onMount } from "svelte";
  import BackupPanel from "./BackupPanel.svelte";
  import PackPanel from "./PackPanel.svelte";
  import {
    command,
    coverage,
    levels,
    type Answer,
    type AppState,
    type Session,
    type Settings,
  } from "./lib/api";

  let app = $state<AppState | null>(null);
  let session = $state<Session | null>(null);
  let page = $state<"learn" | "progress" | "settings" | "lesson">("learn");
  let busy = $state(false);
  let loading = $state(true);
  let error = $state("");
  let notice = $state("");
  let text = $state("");
  let option = $state("");
  let tiles = $state<string[]>([]);
  let report = $state("");
  let reportOpen = $state(false);
  const defaultSettings = (): Settings => ({
    level: "Beginner",
    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    dailyGoal: 10,
    theme: "system",
    reducedMotion: false,
    textScale: 100,
  });
  let settings = $state<Settings>(defaultSettings());
  let draftWrites: Promise<void> = Promise.resolve();
  let draftFailure = "";
  let focusTarget = $state<HTMLHeadingElement>();

  const selectedCoverage = $derived(
    coverage[app?.settings?.level ?? "Beginner"],
  );
  const exercise = $derived(session?.exercise);
  const canCheck = $derived(
    exercise?.kind === "ordering"
      ? tiles.length > 0
      : exercise?.kind === "choice" || exercise?.kind === "reading"
        ? !!option
        : !!text.trim(),
  );
  const goalPercent = $derived(
    app?.settings
      ? Math.min(100, (app.progress.todayXp / app.settings.dailyGoal) * 100)
      : 0,
  );

  $effect(() => {
    const current = app?.settings ?? defaultSettings();
    document.documentElement.dataset.theme = current.theme;
    document.documentElement.dataset.reducedMotion = String(
      current.reducedMotion,
    );
    document.documentElement.style.fontSize = `${current.textScale}%`;
  });

  async function load() {
    loading = true;
    error = "";
    try {
      app = await command<AppState>("app_state");
      if (app.settings) settings = { ...app.settings };
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  onMount(() => {
    void load();
  });

  async function act(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = "";
    notice = "";
    try {
      await action();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  function setSession(next: Session) {
    session = next;
    const draft = next.exercise?.draft;
    text = draft?.type === "text" ? draft.value : "";
    option = draft?.type === "option" ? draft.value : "";
    tiles = draft?.type === "tiles" ? [...draft.value] : [];
    reportOpen = false;
    report = "";
    page = "lesson";
  }
  function response(): Answer {
    if (exercise?.kind === "ordering")
      return { type: "tiles", value: [...tiles] };
    if (exercise?.kind === "choice" || exercise?.kind === "reading")
      return { type: "option", value: option };
    return { type: "text", value: text };
  }
  function queueDraft() {
    if (!session || !exercise) return;
    const args = {
      sessionId: session.id,
      exerciseId: exercise.id,
      answer: response(),
    };
    draftWrites = draftWrites
      .then(async () => {
        await command("save_draft", args);
        draftFailure = "";
      })
      .catch((e) => {
        draftFailure = String(e);
        error = `Your draft could not be saved: ${draftFailure}`;
      });
  }
  async function flushDraft() {
    await draftWrites;
    if (draftFailure) throw new Error(draftFailure);
  }
  async function saveSettings() {
    app = await command<AppState>("save_settings", { settings });
    notice = "Study settings saved on this computer.";
    page = "learn";
  }
  async function begin(lessonId: string) {
    setSession(await command<Session>("start_lesson", { lessonId }));
  }
  async function leaveLesson() {
    await flushDraft();
    app = await command<AppState>("app_state");
    page = "learn";
  }
  async function lessonCommand(
    name: string,
    extra: Record<string, unknown> = {},
  ) {
    if (!session || !exercise) return;
    await flushDraft();
    const previous = session.position;
    setSession(
      await command<Session>(name, {
        sessionId: session.id,
        exerciseId: exercise.id,
        ...extra,
      }),
    );
    if (session.status === "completed")
      app = await command<AppState>("app_state");
    if (session.position !== previous || session.status === "completed") {
      requestAnimationFrame(() => focusTarget?.focus());
    }
  }
  function chooseTile(id: string) {
    tiles = [...tiles, id];
    queueDraft();
  }
  function removeTile(id: string) {
    tiles = tiles.filter((value) => value !== id);
    queueDraft();
  }
</script>

<svelte:head><title>Italian learning · Development</title></svelte:head>

<a class="skip-link" href="#main">Skip to content</a>
<div class="app-shell">
  <aside class="sidebar" aria-label="Application navigation">
    <div class="identity">
      <div class="monogram" aria-hidden="true">it<span>·</span></div>
      <div>
        <strong>Italian learning</strong><small>Development build</small>
      </div>
    </div>
    <div class="nav-caption">YOUR STUDY SPACE</div>
    <nav aria-label="Main">
      {#each [{ id: "learn", symbol: "▦", label: "Learn" }, { id: "progress", symbol: "↗", label: "Progress" }, { id: "settings", symbol: "⚙", label: "Settings" }] as item}
        <button
          class:active={page === item.id ||
            (item.id === "learn" && page === "lesson")}
          aria-current={page === item.id ? "page" : undefined}
          disabled={busy || loading || !app?.settings}
          onclick={() =>
            act(async () => {
              if (page === "lesson") await leaveLesson();
              page = item.id as typeof page;
              notice = "";
            })}
        >
          <span aria-hidden="true">{item.symbol}</span>{item.label}
        </button>
      {/each}
    </nav>
    <div class="sidebar-note">
      <span class="status-dot" aria-hidden="true"></span><strong
        >Local study</strong
      >
      <p>One profile, stored on this computer.</p>
      <small>Offline foundation · milestone 1</small>
    </div>
  </aside>

  <main id="main" tabindex="-1">
    <header class="topbar">
      <span>YOUR ITALIAN, AT YOUR PACE</span><span class="build-tag"
        >In development</span
      >
    </header>
    {#if error}<div class="message error" role="alert">
        <strong>Something needs attention</strong>
        <p>{error}</p>
      </div>{/if}
    {#if notice}<div class="message" role="status">{notice}</div>{/if}

    {#if loading}
      <div class="empty-state" role="status">
        <span class="eyebrow">GETTING READY</span>
        <h1>Opening your study space…</h1>
      </div>
    {:else if !app}
      <div class="empty-state">
        <h1>Your study space could not open</h1>
        <p>The desktop app needs its local database connection.</p>
        <button class="primary" onclick={load}>Try again</button>
      </div>
    {:else if !app.settings || page === "settings"}
      <section class="settings-page">
        <span class="eyebrow"
          >{app.settings ? "MAKE IT YOURS" : "BENVENUTO · WELCOME"}</span
        >
        <h1>
          {app.settings
            ? "Your study settings"
            : "A place to begin your Italian."}
        </h1>
        <p class="lede">
          {app.settings
            ? "Change your starting point without erasing earlier progress."
            : "Choose a starting point. You can change it later and explore earlier topics any time."}
        </p>
        {#if !app.settings}<div class="callout">
            <strong>Your learning stays on this computer.</strong>
            <p>
              No login is needed for offline study. AI conversations and shared
              leaderboards will require a connection when implemented.
            </p>
            <p>
              {app.lessons.some(
                (lesson) => lesson.editorialStatus === "approved",
              )
                ? "Reviewed lessons are available for offline study."
                : "No reviewed lessons are currently active. Development lessons are for testing only."}
            </p>
          </div>{/if}
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void act(saveSettings);
          }}
        >
          <fieldset>
            <legend>Where would you like to start?</legend>
            <div class="level-grid">
              {#each levels as level}<button
                  type="button"
                  class:selected={settings.level === level}
                  aria-pressed={settings.level === level}
                  onclick={() => (settings.level = level)}
                  >{level}<small
                    >{level === "Beginner" ? "Start here" : "CEFR"}</small
                  ></button
                >{/each}
            </div>
          </fieldset>
          <div class="form-grid">
            <label
              >Daily XP goal<input
                type="number"
                min="1"
                max="4294967295"
                step="1"
                bind:value={settings.dailyGoal}
                required
              /><small>Default: 10 XP. Draft fixtures do not earn XP.</small
              ></label
            >
            <label
              >Study time zone<input
                bind:value={settings.timeZone}
                required
                placeholder="America/Chicago"
              /><small
                >Used for daily credit and streaks. Existing study dates stay
                unchanged.</small
              ></label
            >
            <div>
              <label for="appearance">Appearance</label><select
                id="appearance"
                bind:value={settings.theme}
                ><option value="system">Use system setting</option><option
                  value="light">Light</option
                ><option value="dark">Dark</option></select
              >
            </div>
            <div>
              <label for="text-size">Text size</label><select
                id="text-size"
                bind:value={settings.textScale}
                ><option value={100}>100%</option><option value={125}
                  >125%</option
                ><option value={150}>150%</option><option value={175}
                  >175%</option
                ><option value={200}>200%</option></select
              >
            </div>
          </div>
          <label class="checkbox"
            ><input type="checkbox" bind:checked={settings.reducedMotion} /> Reduce
            motion</label
          >
          <p class="muted">
            Optional placement, reminders, and downloads are planned in the
            approved spec and are not implemented in this build.
          </p>
          <button class="primary" disabled={busy}
            >{busy
              ? "Saving…"
              : app.settings
                ? "Save settings"
                : "Open my study space"}<span aria-hidden="true">
              →</span
            ></button
          >
        </form>
        <PackPanel
          {busy}
          run={act}
          onChanged={async () => {
            app = await command<AppState>("app_state");
          }}
        />
        <BackupPanel
          {busy}
          run={act}
          onRestored={(state) => {
            app = state;
            session = null;
            settings = state.settings
              ? { ...state.settings }
              : defaultSettings();
            page = "settings";
          }}
        />
      </section>
    {:else if page === "learn"}
      {#each app.contentIssues as issue}<p class="callout" role="alert">
          {issue} Open Settings to check or repair installed packs.
        </p>{/each}
      <section class="page-heading">
        <div>
          <span class="eyebrow">ONE STEP AT A TIME</span>
          <h1>Your learning path</h1>
          <p class="lede">A steady habit. A little more confidence.</p>
        </div>
        <span class="level-chip"
          >{app.settings.level}{app.settings.level === "Beginner"
            ? ""
            : " · CEFR"}</span
        >
      </section>
      <div class="learn-layout">
        <div class="main-column">
          <section class="course-banner">
            <div>
              <span class="eyebrow">YOUR STARTING POINT</span>
              <h2>{selectedCoverage.title}</h2>
              <p>{selectedCoverage.description}</p>
              <button class="text-button" onclick={() => (page = "settings")}
                >Change starting level <span aria-hidden="true">↗</span></button
              >
            </div>
            <div class="banner-art" aria-hidden="true">
              <div class="sun"></div>
              <div class="arch"></div>
              <div class="step one"></div>
              <div class="step two"></div>
              <div class="step three"></div>
            </div>
          </section>
          <section class="panel course-empty">
            <div class="section-label">
              <span class="number">01</span><span>COURSE LIBRARY</span>
            </div>
            <h2>Your course begins here</h2>
            {#if app.lessons.some((lesson) => lesson.editorialStatus === "approved")}
              <p>
                Reviewed lessons available for offline study. You can choose any
                installed topic; this list does not certify full course
                coverage.
              </p>
              {#each app.lessons.filter((lesson) => lesson.editorialStatus === "approved") as lesson}
                <div class="fixture-row">
                  <div>
                    <strong>{lesson.title}</strong><small
                      >{lesson.level} · {lesson.exerciseCount} exercises</small
                    >
                    <p>{lesson.objective}</p>
                  </div>
                  <button
                    class="primary"
                    disabled={busy || !!app.activeSession}
                    onclick={() => act(() => begin(lesson.id))}
                    >Start {lesson.title}</button
                  >
                </div>
              {/each}
            {:else}<p>
                No verified, reviewed lessons are active. The beginner–C2
                curriculum, audio, and speaking assets still need authoring and
                validation.
              </p>
              <span class="plain-tag">Content review required</span>
            {/if}
          </section>
          {#if app.activeSession}<section class="panel resume">
              <span class="eyebrow">SAVED ON THIS COMPUTER</span>
              <h2>Pick up where you left off</h2>
              <p>
                {app.activeSession.lesson.title} · item {app.activeSession
                  .position + 1} of {app.activeSession.lesson.exerciseCount}
              </p>
              <button
                class="primary"
                disabled={busy}
                onclick={() => {
                  if (app?.activeSession) setSession(app.activeSession);
                }}>Resume saved lesson →</button
              >
            </section>{/if}
          {#if app.developmentBuild}
            <section class="panel development">
              <div class="section-label">
                <span class="dev-dot" aria-hidden="true"></span>DEVELOPMENT
                WORKBENCH
              </div>
              <h2>Try the offline lesson flow</h2>
              <p>
                Unreviewed development material. Exercises, corrections, hints,
                and drafts save locally. This fixture earns no XP, streak,
                achievement, or course credit.
              </p>
              {#each app.lessons.filter((lesson) => lesson.editorialStatus === "draft") as lesson}<div
                  class="fixture-row"
                >
                  <div>
                    <strong>{lesson.title}</strong><small
                      >{lesson.exerciseCount} exercises · {lesson.level} draft lesson</small
                    >
                  </div>
                  <button
                    class="secondary"
                    disabled={busy || !!app.activeSession}
                    onclick={() => act(() => begin(lesson.id))}
                    >Open test lesson <span aria-hidden="true">→</span></button
                  >
                </div>{/each}
            </section>
          {/if}
        </div>
        <aside class="daily-column" aria-label="Daily progress">
          <section class="panel goal-card">
            <span class="eyebrow">TODAY'S SMALL STEP</span>
            <div class="goal-number">
              {app.progress.todayXp}<span> / {app.settings.dailyGoal} XP</span>
            </div>
            <progress aria-label="Daily XP goal" value={goalPercent} max="100"
            ></progress>
            <p>
              {app.progress.todayXp >= app.settings.dailyGoal
                ? "Daily goal reached."
                : "Every completed lesson is a step forward."}
            </p>
            <div class="stat-line">
              <span>Current streak</span><strong
                >{app.progress.currentStreak} days</strong
              >
            </div>
            <div class="stat-line">
              <span>Total earned</span><strong>{app.progress.totalXp} XP</strong
              >
            </div>
          </section>
          <div class="quiet-note">
            <span aria-hidden="true">✦</span>
            <p>Practice freely. No hearts, no lives, no limits on retries.</p>
          </div>
        </aside>
      </div>
    {:else if page === "progress"}
      <span class="eyebrow">THE BIGGER PICTURE</span>
      <h1>Your progress</h1>
      <p class="lede">
        Completion, first-attempt evidence, and mastery are different measures.
      </p>
      <div class="stats-grid">
        {#each [{ label: "Earned XP", value: app.progress.totalXp }, { label: "Completed lessons", value: app.progress.completedLessons }, { label: "Current streak", value: app.progress.currentStreak }, { label: "Best streak", value: app.progress.bestStreak }] as stat}<section
            class="panel"
          >
            <span class="eyebrow">{stat.label}</span>
            <div class="big-stat">{stat.value}</div>
          </section>{/each}
      </div>
      <section class="panel">
        <h2>Achievements</h2>
        {#if app.progress.achievements.length}<ul>
            {#each app.progress.achievements as achievement}<li>
                {achievement === "first_lesson"
                  ? "First lesson · complete a reviewed lesson"
                  : achievement === "100_xp"
                    ? "100 XP · earn 100 XP"
                    : "Seven-day streak · study seven consecutive days"}
              </li>{/each}
          </ul>{:else}<p>
            Your achievements will appear here as you study reviewed content.
            Development fixtures do not count.
          </p>{/if}
      </section>
      <section class="panel">
        <h2>Skill evidence</h2>
        <p>
          Full skill-level reporting is still being developed. This build does
          not assess CEFR proficiency. Review scheduling, listening, speaking,
          and advanced writing are still to be implemented.
        </p>
      </section>
    {:else if page === "lesson" && session}
      <div class="lesson-top">
        <button
          class="text-button"
          disabled={busy}
          onclick={() => act(leaveLesson)}
          >← {session.status === "completed"
            ? "Back to learning"
            : "Save & exit"}</button
        ><span class="plain-tag"
          >{session.practiceOnly
            ? "Draft fixture · no course credit"
            : session.lesson.level}</span
        >
      </div>
      {#if session.status === "completed"}
        <section class="summary">
          <div class="completion-mark" aria-hidden="true">✓</div>
          <span class="eyebrow">SESSION SAVED</span>
          <h1 tabindex="-1" bind:this={focusTarget}>
            {session.practiceOnly ? "Test lesson complete" : "Lesson complete"}
          </h1>
          <p class="lede">{session.lesson.title}</p>
          <div class="stats-grid">
            <div class="panel">
              <span class="eyebrow">First answer correct</span>
              <div class="big-stat">
                {session.firstCorrect}<small>
                  / {session.lesson.exerciseCount}</small
                >
              </div>
            </div>
            <div class="panel">
              <span class="eyebrow">Independent first answers</span>
              <div class="big-stat">
                {session.independentCorrect}<small>
                  / {session.lesson.exerciseCount}</small
                >
              </div>
            </div>
          </div>
          <p>
            {session.hintedItems} items with hints · {session.revealedItems} solutions
            reviewed
          </p>
          <div class="callout">
            {session.practiceOnly
              ? "This unreviewed test fixture earns no XP or learning credit. Its evidence is kept only for development testing."
              : `${session.xpEarned} XP earned. A specific lesson earns XP at most once per study day. Completion does not establish mastery.`}
          </div>
          <button
            class="primary"
            disabled={busy}
            onclick={() => act(leaveLesson)}>Back to my path →</button
          >
        </section>
      {:else if exercise}
        <section class="lesson-body" aria-busy={busy}>
          <div class="lesson-progress">
            <progress
              aria-label="Lesson progress"
              value={session.position}
              max={session.lesson.exerciseCount}
            ></progress><small
              >{session.position + 1} / {session.lesson.exerciseCount}</small
            >
          </div>
          <span class="eyebrow"
            >{session.lesson.title} · {exercise.kind === "cloze"
              ? "Fill in the blank"
              : exercise.kind}</span
          >
          <h1 tabindex="-1" bind:this={focusTarget}>{exercise.instruction}</h1>
          {#if exercise.context}<blockquote lang="it">
              {exercise.context}
            </blockquote>{/if}
          <p
            class="exercise-prompt"
            lang={exercise.kind === "translation" ||
            exercise.kind === "ordering" ||
            exercise.kind === "reading"
              ? "en"
              : "it"}
          >
            {exercise.prompt}
          </p>
          <form
            onsubmit={(event) => {
              event.preventDefault();
              if (exercise?.readyToContinue)
                void act(() => lessonCommand("advance_session"));
              else if (canCheck)
                void act(() =>
                  lessonCommand("submit_answer", {
                    answer: response(),
                    requestId: crypto.randomUUID(),
                  }),
                );
            }}
          >
            {#if exercise.kind === "choice" || exercise.kind === "reading"}
              <fieldset class="answer-options">
                <legend class="sr-only">Choose your answer</legend
                >{#each exercise.options as choice, i}<label
                    class:chosen={option === choice.id}
                    ><input
                      type="radio"
                      name="answer"
                      value={choice.id}
                      bind:group={option}
                      disabled={busy || exercise.readyToContinue}
                      onchange={queueDraft}
                    /><span class="option-index" aria-hidden="true"
                      >{i + 1}</span
                    ><span>{choice.text}</span></label
                  >{/each}
              </fieldset>
            {:else if exercise.kind === "ordering"}
              <div class="tile-answer" role="group" aria-label="Your sentence">
                {#if !tiles.length}<span class="muted"
                    >Choose words below to build your sentence.</span
                  >{/if}{#each tiles as id}<button
                    type="button"
                    disabled={busy || exercise.readyToContinue}
                    aria-label={`Remove ${exercise.options.find((item) => item.id === id)?.text}`}
                    onclick={() => removeTile(id)}
                    >{exercise.options.find((item) => item.id === id)
                      ?.text}<span aria-hidden="true"> ×</span></button
                  >{/each}
              </div>
              <div class="tile-bank" role="group" aria-label="Available words">
                {#each exercise.options.filter((item) => !tiles.includes(item.id)) as tile}<button
                    type="button"
                    disabled={busy || exercise.readyToContinue}
                    onclick={() => chooseTile(tile.id)}>{tile.text}</button
                  >{/each}
              </div>
            {:else}
              <label class="answer-label"
                >Your answer<textarea
                  rows="3"
                  bind:value={text}
                  disabled={busy || exercise.readyToContinue}
                  oninput={queueDraft}
                  maxlength="1000"
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  lang="it"
                  placeholder="Type in Italian…"></textarea></label
              >
              <div class="accent-keys" aria-label="Italian accented letters">
                {#each ["à", "è", "é", "ì", "ò", "ù"] as accent}<button
                    type="button"
                    aria-label={`Insert ${accent}`}
                    disabled={busy || exercise.readyToContinue}
                    onclick={() => {
                      text += accent;
                      queueDraft();
                    }}>{accent}</button
                  >{/each}
              </div>
            {/if}
            {#if exercise.hint}<div class="callout hint" role="status">
                <strong>Hint</strong>
                <p>{exercise.hint}</p>
                <small>Hints are recorded with this exercise.</small>
              </div>{/if}
            <div aria-live="polite" aria-atomic="true">
              {#if exercise.feedback}<div
                  class="feedback"
                  class:correct={exercise.feedback.correct}
                >
                  <strong
                    >{exercise.feedback.revealed
                      ? "Solution reviewed"
                      : exercise.feedback.correct
                        ? "Correct"
                        : "Try again — you have unlimited retries"}</strong
                  >
                  <p>{exercise.feedback.explanation}</p>
                  {#if !exercise.feedback.correct}<p>
                      <span class="muted">Expected answer:</span>
                      {exercise.feedback.expected}
                    </p>{/if}<small
                    >Your first result stays in the record.</small
                  >
                </div>{/if}
            </div>
            <div class="lesson-actions">
              <div>
                <button
                  type="button"
                  class="text-button"
                  disabled={busy || exercise.readyToContinue || !!exercise.hint}
                  onclick={() => act(() => lessonCommand("show_hint"))}
                  >Show hint</button
                ><button
                  type="button"
                  class="text-button"
                  disabled={busy || exercise.readyToContinue}
                  onclick={() => act(() => lessonCommand("reveal_answer"))}
                  >Review solution</button
                >
              </div>
              <button
                class="primary"
                disabled={busy || (!exercise.readyToContinue && !canCheck)}
                >{busy
                  ? "Saving…"
                  : exercise.readyToContinue
                    ? session.position + 1 === session.lesson.exerciseCount
                      ? "Finish lesson →"
                      : "Continue →"
                    : "Check answer"}</button
              >
            </div>
          </form>
          {#if exercise.feedback && !exercise.feedback.correct}<button
              class="text-button report-link"
              onclick={() => (reportOpen = !reportOpen)}
              >My answer should be accepted</button
            >{/if}
          {#if reportOpen}<form
              class="report-form"
              onsubmit={(event) => {
                event.preventDefault();
                void act(async () => {
                  await command("report_answer", {
                    sessionId: session!.id,
                    exerciseId: exercise!.id,
                    note: report,
                  });
                  reportOpen = false;
                  notice =
                    "Report saved locally for editorial review. It has not been sent anywhere.";
                });
              }}
            >
              <label
                >Describe your answer and why it should be accepted<textarea
                  bind:value={report}
                  required
                  maxlength="1000"></textarea></label
              ><button class="secondary" disabled={busy}
                >Save local report</button
              >
            </form>{/if}
        </section>
      {/if}
    {/if}
    <footer>
      Free, open source, and ad-free. <span
        >Early development · features and course content are incomplete.</span
      >
    </footer>
  </main>
</div>
