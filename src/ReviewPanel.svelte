<script lang="ts">
  import { onMount } from "svelte";
  import {
    command,
    type Answer,
    type ReviewOverview,
    type ReviewSession,
  } from "./lib/api";
  let {
    busy,
    run,
    hasLesson,
    onChanged,
    onResumeLesson,
  }: {
    busy: boolean;
    run: (action: () => Promise<void>) => Promise<void>;
    hasLesson: boolean;
    onChanged: () => Promise<void>;
    onResumeLesson: () => void;
  } = $props();
  let overview = $state<ReviewOverview | null>(null);
  let session = $state<ReviewSession | null>(null);
  let text = $state("");
  let option = $state("");
  let tiles = $state<string[]>([]);
  let easy = $state(false);
  let endConfirm = $state(false);
  let failure = $state("");
  let heading = $state<HTMLHeadingElement>();
  let writes: Promise<void> = Promise.resolve();
  let writeFailure = "";
  const exercise = $derived(session?.exercise);
  const canCheck = $derived(
    exercise?.kind === "ordering"
      ? tiles.length > 0
      : exercise?.kind === "choice" || exercise?.kind === "reading"
        ? !!option
        : !!text.trim(),
  );
  function setSession(s: ReviewSession) {
    session = s;
    easy = false;
    endConfirm = false;
    const draft = s.exercise?.draft;
    text = draft?.type === "text" ? draft.value : "";
    option = draft?.type === "option" ? draft.value : "";
    tiles = draft?.type === "tiles" ? [...draft.value] : [];
  }
  async function refresh() {
    overview = await command<ReviewOverview>("review_overview");
  }
  onMount(() => {
    void refresh().catch((e) => (failure = String(e)));
  });
  function answer(): Answer {
    if (exercise?.kind === "ordering")
      return { type: "tiles", value: [...tiles] };
    if (exercise?.kind === "choice" || exercise?.kind === "reading")
      return { type: "option", value: option };
    return { type: "text", value: text };
  }
  function draft() {
    if (!session || !exercise) return;
    const args = {
      sessionId: session.id,
      exerciseId: exercise.id,
      answer: answer(),
    };
    writes = writes
      .then(async () => {
        await command("review_draft", args);
        writeFailure = "";
      })
      .catch((e) => {
        writeFailure = String(e);
        failure = `Your review draft could not be saved: ${writeFailure}`;
      });
  }
  async function flush() {
    await writes;
    if (writeFailure) throw new Error(writeFailure);
  }
  async function action(name: string, extra: Record<string, unknown> = {}) {
    if (!session || !exercise) return;
    await flush();
    const previous = session.position;
    setSession(
      await command<ReviewSession>(name, {
        sessionId: session.id,
        exerciseId: exercise.id,
        ...extra,
      }),
    );
    await onChanged();
    if (previous !== session.position)
      requestAnimationFrame(() => heading?.focus());
  }
  async function start(practice: boolean) {
    setSession(await command<ReviewSession>("start_review", { practice }));
    await onChanged();
  }
  async function exit() {
    await flush();
    session = null;
    await refresh();
    await onChanged();
  }
</script>

{#if failure}<p class="message error" role="alert">{failure}</p>{/if}
{#if !session}
  <span class="eyebrow">KEEP WHAT YOU LEARN</span>
  <h1>Review</h1>
  <p class="lede">Revisit items from your completed lesson interactions.</p>
  {#if !overview}
    <p role="status">Checking your saved review items…</p>
    {#if failure}<button
        class="secondary"
        disabled={busy}
        onclick={() => run(refresh)}>Try again</button
      >{/if}
  {:else if overview.active}
    <section class="panel resume">
      <h2>Your review is saved</h2>
      <p>
        Item {overview.active.position + 1} of {overview.active.itemCount}. Your
        drafts and first answers are kept.
      </p>
      <button
        class="primary"
        disabled={busy}
        onclick={() => setSession(overview!.active!)}
        >Resume saved review →</button
      >
    </section>
  {:else}
    <section class="panel">
      <h2>
        {overview.dueCount}
        {overview.dueCount === 1 ? "item due" : "items due"}
      </h2>
      <p>
        {overview.availableCount} eligible items from active, independently reviewed
        content.
      </p>
      {#if overview.availableCount === 0}
        <p>
          Complete a lesson from an independently reviewed pack to introduce
          review items. Draft development lessons do not create review items or
          earn XP.
        </p>
        <p>There is no reviewed course content bundled in this build.</p>
      {:else}
        <p>
          A session contains up to {overview.sessionLimit} distinct items. Due items
          come first. Practice also includes items scheduled for later.
        </p>
        <div class="lesson-actions">
          <button
            class="primary"
            disabled={busy || hasLesson || overview.dueCount === 0}
            onclick={() => run(() => start(false))}>Review due items →</button
          >
          <button
            class="secondary"
            disabled={busy || hasLesson}
            onclick={() => run(() => start(true))}
            >Practice available items</button
          >
        </div>
      {/if}
    </section>
  {/if}
  {#if hasLesson}<section class="panel resume">
      <h2>A lesson is waiting</h2>
      <p>Finish your saved lesson before starting a review.</p>
      <button class="secondary" disabled={busy} onclick={onResumeLesson}
        >Resume saved lesson →</button
      >
    </section>{/if}
  <section class="panel">
    <h2>How review credit works</h2>
    <p>
      Complete every item, including corrections or reviewing the solution, to
      finish a session. Review earns 5 XP at most once per study day. Further
      practice is unrestricted.
    </p>
    <p>
      Hints and corrections count as assisted answers. Easy is an optional
      self-rating after an independent correct answer. Scheduling and XP do not
      certify mastery or a CEFR level.
    </p>
  </section>
{:else if session.status !== "active"}
  <section class="summary">
    <span class="eyebrow">REVIEW SAVED</span>
    <h1 tabindex="-1" bind:this={heading}>
      {session.status === "completed" ? "Review complete" : "Review ended"}
    </h1>
    <p class="lede">{session.xpEarned} XP earned.</p>
    <p>
      {session.status === "completed"
        ? "Review XP is awarded once per study day. You can keep practicing."
        : "Unfinished reviews earn no XP. Ratings already saved for completed items are kept."}
    </p>
    <button class="primary" disabled={busy} onclick={() => run(exit)}
      >Back to review →</button
    >
  </section>
{:else if exercise}
  <div class="lesson-top">
    <button class="text-button" disabled={busy} onclick={() => run(exit)}
      >← Save & exit</button
    ><span class="plain-tag">Review · {session.target?.skill}</span>
  </div>
  <section class="lesson-body" aria-busy={busy}>
    <div class="lesson-progress">
      <progress
        aria-label="Review progress"
        value={session.position}
        max={session.itemCount}
      ></progress><small>{session.position + 1} / {session.itemCount}</small>
    </div>
    <h1 tabindex="-1" bind:this={heading}>{exercise.instruction}</h1>
    {#if exercise.context}<blockquote lang="it">
        {exercise.context}
      </blockquote>{/if}
    <p class="exercise-prompt">{exercise.prompt}</p>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void run(() =>
          exercise?.readyToContinue
            ? action("advance_review", { easy })
            : action("review_submit", {
                answer: answer(),
                requestId: crypto.randomUUID(),
              }),
        );
      }}
    >
      {#if exercise.kind === "choice" || exercise.kind === "reading"}
        <fieldset class="answer-options">
          <legend class="sr-only">Choose your answer</legend>
          {#each exercise.options as choice, i}<label
              class:chosen={option === choice.id}
              ><input
                type="radio"
                name="review-answer"
                value={choice.id}
                bind:group={option}
                disabled={busy || exercise.readyToContinue}
                onchange={draft}
              /><span class="option-index" aria-hidden="true">{i + 1}</span
              ><span>{choice.text}</span></label
            >{/each}
        </fieldset>
      {:else if exercise.kind === "ordering"}
        <div class="tile-answer" role="group" aria-label="Your sentence">
          {#each tiles as id}<button
              type="button"
              disabled={busy || exercise.readyToContinue}
              aria-label={`Remove ${exercise.options.find((o) => o.id === id)?.text}`}
              onclick={() => {
                tiles = tiles.filter((t) => t !== id);
                draft();
              }}>{exercise.options.find((o) => o.id === id)?.text} ×</button
            >{/each}
        </div>
        <div class="tile-bank" role="group" aria-label="Available words">
          {#each exercise.options.filter((o) => !tiles.includes(o.id)) as tile}<button
              type="button"
              disabled={busy || exercise.readyToContinue}
              onclick={() => {
                tiles = [...tiles, tile.id];
                draft();
              }}>{tile.text}</button
            >{/each}
        </div>
      {:else}
        <label class="answer-label"
          >Your answer<textarea
            rows="3"
            bind:value={text}
            oninput={draft}
            disabled={busy || exercise.readyToContinue}
            maxlength="1000"
            autocomplete="off"
            spellcheck="false"
            lang="it"></textarea></label
        >
        <div class="accent-keys" aria-label="Italian accented letters">
          {#each ["à", "è", "é", "ì", "ò", "ù"] as accent}<button
              type="button"
              aria-label={`Insert ${accent}`}
              disabled={busy || exercise.readyToContinue}
              onclick={() => {
                text += accent;
                draft();
              }}>{accent}</button
            >{/each}
        </div>
      {/if}
      {#if exercise.hint}<div class="callout hint" role="status">
          <strong>Hint</strong>
          <p>{exercise.hint}</p>
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
                  : "Try again — unlimited retries"}</strong
            >
            <p>{exercise.feedback.explanation}</p>
            {#if !exercise.feedback.correct}<p>
                Expected answer: {exercise.feedback.expected}
              </p>{/if}<small>Your first result stays in the record.</small>
          </div>{/if}
      </div>
      {#if session.canRateEasy}<label
          ><input type="checkbox" bind:checked={easy} disabled={busy} /> This felt
          easy · use my explicit Easy rating</label
        >{/if}
      <div class="lesson-actions">
        <div>
          <button
            type="button"
            class="text-button"
            disabled={busy || exercise.readyToContinue || !!exercise.hint}
            onclick={() => run(() => action("review_hint"))}>Show hint</button
          >
          <button
            type="button"
            class="text-button"
            disabled={busy || exercise.readyToContinue}
            onclick={() => run(() => action("review_reveal"))}
            >Review solution</button
          >
        </div>
        <button
          class="primary"
          disabled={busy || (!exercise.readyToContinue && !canCheck)}
          >{exercise.readyToContinue
            ? session.position + 1 === session.itemCount
              ? "Finish review →"
              : "Continue →"
            : "Check answer"}</button
        >
      </div>
    </form>
    <button
      class="text-button"
      disabled={busy}
      onclick={() => (endConfirm = true)}
      >End this review without completing</button
    >
    {#if endConfirm}<div class="callout">
        <p>
          End the saved review? No XP will be awarded. Ratings already saved for
          completed items remain.
        </p>
        <button
          class="secondary"
          disabled={busy}
          onclick={() =>
            run(async () => {
              await flush();
              setSession(
                await command<ReviewSession>("abort_review", {
                  sessionId: session!.id,
                }),
              );
              await onChanged();
            })}>End review</button
        >
        <button
          class="text-button"
          disabled={busy}
          onclick={() => (endConfirm = false)}>Keep reviewing</button
        >
      </div>{/if}
  </section>
{/if}
