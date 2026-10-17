<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    command,
    type AppState,
    type BackupTicket,
    type RecoveryEntry,
  } from "./lib/api";
  let {
    busy,
    run,
    onRestored,
  }: {
    busy: boolean;
    run: (action: () => Promise<void>) => Promise<void>;
    onRestored: (state: AppState) => void;
  } = $props();
  let ticket = $state<BackupTicket | null>(null);
  let confirmed = $state(false);
  let message = $state("");
  let listError = $state("");
  let recovery = $state<RecoveryEntry[]>([]);
  let previewHeading = $state<HTMLHeadingElement>();

  async function refresh() {
    try {
      recovery = await command<RecoveryEntry[]>("list_recovery_backups");
      listError = "";
    } catch (e) {
      listError = String(e);
    }
  }
  onMount(() => {
    void refresh();
  });
  onDestroy(() => {
    if (ticket) void command("cancel_backup_import").catch(() => {});
  });
  function show(next: BackupTicket | null) {
    ticket = next;
    confirmed = false;
    message = next
      ? ""
      : "File selection cancelled. Your study data is unchanged.";
    if (next) requestAnimationFrame(() => previewHeading?.focus());
  }
  async function choose() {
    ticket = null;
    confirmed = false;
    message = "";
    show(await command<BackupTicket | null>("choose_learner_backup"));
  }
  async function previewRecovery(id: string) {
    ticket = null;
    confirmed = false;
    message = "";
    show(await command<BackupTicket>("preview_recovery_backup", { id }));
  }
  async function exportBackup() {
    const path = await command<string | null>("export_learner_backup");
    message = path
      ? `Backup saved to ${path}`
      : "Export cancelled. Your study data is unchanged.";
  }
  async function cancel() {
    await command("cancel_backup_import");
    ticket = null;
    confirmed = false;
    message = "Restore cancelled. Your study data is unchanged.";
  }
  async function restore() {
    if (!ticket || !confirmed) return;
    const token = ticket.token;
    ticket = null;
    confirmed = false;
    const result = await command<{ state: AppState; recoveryPath: string }>(
      "restore_learner_backup",
      { token },
    );
    onRestored(result.state);
    message = `Backup restored. A pre-import recovery copy was saved to ${result.recoveryPath}`;
    await refresh();
  }
</script>

<section class="panel backups-panel" aria-label="Backups and recovery">
  <span class="eyebrow">SAVED ON YOUR COMPUTER</span>
  <h2>Backups &amp; recovery</h2>
  <p>
    Export your saved settings, progress, lesson drafts, review item histories
    and local answer reports. Backups are local, unencrypted files; choose where
    to keep them. Media files are excluded.
  </p>
  <div class="backup-actions">
    <button
      class="secondary"
      disabled={busy || !!ticket}
      onclick={() => run(exportBackup)}>Export backup</button
    >
    <button class="secondary" disabled={busy} onclick={() => run(choose)}
      >Choose backup to restore</button
    >
  </div>
  {#if message}<p class="message backup-message" role="status">
      {message}
    </p>{/if}
  {#if ticket}
    <section class="restore-preview" aria-label="Restore preview">
      <h3 tabindex="-1" bind:this={previewHeading}>Review before restoring</h3>
      <dl>
        <div>
          <dt>Saved</dt>
          <dd>{new Date(ticket.preview.createdAt).toLocaleString()}</dd>
        </div>
        <div>
          <dt>Starting level</dt>
          <dd>{ticket.preview.level ?? "No profile selected"}</dd>
        </div>
        <div>
          <dt>Recorded sessions</dt>
          <dd>{ticket.preview.sessionCount}</dd>
        </div>
        <div>
          <dt>Earned XP</dt>
          <dd>{ticket.preview.totalXp}</dd>
        </div>
        <div>
          <dt>Completed lessons</dt>
          <dd>{ticket.preview.completedLessons}</dd>
        </div>
        <div>
          <dt>Saved review sessions</dt>
          <dd>{ticket.preview.reviewSessionCount}</dd>
        </div>
        <div>
          <dt>Review item histories</dt>
          <dd>{ticket.preview.reviewItemCount}</dd>
        </div>
      </dl>
      <p>
        Referenced pack versions: {ticket.preview.packReferences.length}.
        Matching versions are already verified on this computer. Restoring does
        not download packs or change the version currently in use.
      </p>
      {#each ticket.preview.packReferences as reference}<p class="muted">
          {reference.packId} · Version {reference.version}
        </p>{/each}
      {#if ticket.preview.activeLesson}<p>
          Saved lesson: {ticket.preview.activeLesson}
        </p>{/if}
      {#if ticket.preview.activeReview}<p>
          Includes an unfinished review session.
        </p>{/if}
      {#if ticket.preview.development}<span class="plain-tag"
          >Development backup · draft credit remains excluded</span
        >{/if}
      <p>
        Restore replaces your current local profile, study history and review
        item histories, including saved review sessions and review XP. A
        verified recovery copy of the existing history is saved before
        replacement.
      </p>
      <label class="checkbox"
        ><input type="checkbox" bind:checked={confirmed} disabled={busy} /> I understand
        this replaces my current local study data.</label
      >
      <div class="backup-actions">
        <button class="secondary" disabled={busy} onclick={() => run(cancel)}
          >Cancel restore</button
        ><button
          class="primary"
          disabled={busy || !confirmed}
          onclick={() => run(restore)}>Restore this backup</button
        >
      </div>
    </section>
  {/if}
  <div class="recovery-list">
    <h3>Recovery copies</h3>
    <p>
      A verified recovery copy is created before a restore or an upgrade of
      existing learner data. Choosing one opens the same validation and
      confirmation flow.
    </p>
    {#if listError}<p role="alert" class="muted">
        Recovery list unavailable: {listError}
      </p>
      <button class="text-button" disabled={busy} onclick={refresh}
        >Retry recovery list</button
      >
    {:else if recovery.length}
      <ul>
        {#each recovery as entry}<li>
            <span
              >{entry.id.startsWith("pre-migrate-")
                ? "Before learner-data upgrade"
                : "Before restore"} · {new Date(
                entry.savedAt,
              ).toLocaleString()}</span
            ><button
              class="text-button"
              disabled={busy}
              onclick={() => run(() => previewRecovery(entry.id))}
              >Preview recovery copy</button
            >
          </li>{/each}
      </ul>
    {:else}<p class="muted">No recovery copies have been created yet.</p>{/if}
  </div>
</section>
