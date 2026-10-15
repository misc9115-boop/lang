<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    command,
    type PackState,
    type PackTicket,
    type PackRecord,
  } from "./lib/api";
  let {
    busy,
    run,
    onChanged,
  }: {
    busy: boolean;
    run: (action: () => Promise<void>) => Promise<void>;
    onChanged: () => Promise<void>;
  } = $props();
  let catalogue = $state<PackState>({ entries: [], issues: [] });
  let ticket = $state<PackTicket | null>(null);
  let confirmed = $state(false);
  let decision = $state<{ action: string; record: PackRecord } | null>(null);
  let message = $state("");
  let error = $state("");
  let heading = $state<HTMLHeadingElement>();
  let chooseButton = $state<HTMLButtonElement>();
  const size = (bytes: number) =>
    `${(bytes / 1024).toLocaleString(undefined, { maximumFractionDigits: 1 })} KiB`;
  async function refresh() {
    try {
      catalogue = await command<PackState>("pack_state");
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
  onMount(() => {
    void refresh();
  });
  onDestroy(() => {
    if (ticket) void command("cancel_pack_install").catch(() => {});
  });
  async function choose() {
    ticket = null;
    decision = null;
    confirmed = false;
    message = "";
    ticket = await command<PackTicket | null>("choose_pack_folder");
    if (!ticket)
      message = "Folder selection cancelled. Installed content is unchanged.";
    else requestAnimationFrame(() => heading?.focus());
  }
  async function cancel() {
    await command("cancel_pack_install");
    ticket = null;
    decision = null;
    confirmed = false;
    message = "Pack change cancelled. Installed content is unchanged.";
    requestAnimationFrame(() => chooseButton?.focus());
  }
  async function install() {
    if (!ticket || !confirmed) return;
    const token = ticket.token;
    ticket = null;
    confirmed = false;
    catalogue = await command<PackState>("install_pack", { token });
    message = "Pack installed. Choose Use version when you want to study it.";
    await onChanged();
    requestAnimationFrame(() => chooseButton?.focus());
  }
  function ask(action: string, record: PackRecord) {
    decision = { action, record };
    confirmed = false;
    message = "";
    requestAnimationFrame(() => heading?.focus());
  }
  async function change() {
    if (!decision || !confirmed) return;
    const { action, record } = decision;
    decision = null;
    confirmed = false;
    catalogue = await command<PackState>("change_pack", {
      action,
      packId: record.entry.packId,
      version: record.entry.version,
    });
    message =
      action === "activate"
        ? "Selected content is ready for offline study."
        : action === "deactivate"
          ? "Pack deactivated. Saved lesson history is unchanged."
          : "Pack payloads removed. Saved lesson history is unchanged.";
    await onChanged();
    requestAnimationFrame(() => chooseButton?.focus());
  }
</script>

<section class="panel" aria-label="Offline lesson packs">
  <span class="eyebrow">AVAILABLE WITHOUT A CONNECTION</span>
  <h2>Offline lesson packs</h2>
  <p>
    Install a local text-pack folder, then choose which version to use.
    Downloads, audio and speech packs are still being developed.
  </p>
  <p class="muted">
    Draft packs are available only in development builds and cannot earn XP or
    course credit. Installing a pack does not approve its content.
  </p>
  <div class="actions">
    <button
      class="secondary"
      disabled={busy || !!ticket || !!decision}
      bind:this={chooseButton}
      onclick={() => run(choose)}>Choose pack folder</button
    >
    <button
      class="secondary"
      disabled={busy || !!ticket || !!decision}
      onclick={() =>
        run(async () => {
          await refresh();
          await onChanged();
        })}>Check installed packs</button
    >
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
  {#each catalogue.issues as issue}<p class="callout" role="alert">
      {issue}
    </p>{/each}
  {#if message}<p role="status">{message}</p>{/if}
  {#if ticket}
    <section class="callout" aria-label="Pack installation preview">
      <h3 tabindex="-1" bind:this={heading}>Review before installing</h3>
      <p><strong>{ticket.title}</strong> · Version {ticket.version}</p>
      <p class="muted">
        License: {ticket.license}<br />Attribution: {ticket.attribution}
      </p>
      <p>
        {ticket.lessonCount}
        {ticket.lessonCount === 1 ? "lesson" : "lessons"} · {size(
          ticket.payloadBytes,
        )} of lesson data · {ticket.editorialStatus === "draft"
          ? "Unreviewed draft"
          : "Reviewed content"}
      </p>
      <p>
        This installs a separate version. Your current selection and saved
        progress stay in place. Actual disk use also includes pack metadata.
      </p>
      <label class="checkbox"
        ><input type="checkbox" bind:checked={confirmed} disabled={busy} /> I
        want to install this pack{ticket.editorialStatus === "draft"
          ? " as unreviewed development content"
          : ""}.</label
      >
      <div class="actions">
        <button
          class="primary"
          disabled={busy || !confirmed}
          onclick={() => run(install)}>Install pack</button
        ><button class="secondary" disabled={busy} onclick={() => run(cancel)}
          >Cancel pack change</button
        >
      </div>
    </section>
  {:else if decision}
    <section class="callout" aria-label="Pack change confirmation">
      <h3 tabindex="-1" bind:this={heading}>
        {decision.action === "activate"
          ? "Use this version?"
          : decision.action === "deactivate"
            ? "Deactivate this pack?"
            : "Remove these payloads?"}
      </h3>
      <p>
        {decision.record.title ?? decision.record.entry.packId} · Version {decision
          .record.entry.version}
      </p>
      <p>
        {decision.action === "remove"
          ? "You will need the original pack folder to reinstall this version. Versions needed by reviewed lesson history cannot be removed."
          : "Saved lessons keep their original exercise snapshots. Your progress will not be reset."}
      </p>
      <label class="checkbox"
        ><input type="checkbox" bind:checked={confirmed} disabled={busy} /> Confirm
        this pack change.</label
      >
      <div class="actions">
        <button
          class="primary"
          disabled={busy || !confirmed}
          onclick={() => run(change)}>Confirm pack change</button
        ><button class="secondary" disabled={busy} onclick={() => run(cancel)}
          >Cancel pack change</button
        >
      </div>
    </section>
  {/if}
  {#if !catalogue.entries.length}<p>
      No packs have been installed in this catalogue.
    </p>{/if}
  <ul class="packs">
    {#each catalogue.entries as record (`${record.entry.packId}:${record.entry.version}`)}
      <li>
        <strong>{record.title ?? record.entry.packId}</strong>
        <p>
          Version {record.entry.version} · {record.entry.active
            ? "In use"
            : record.entry.stored
              ? "Stored"
              : "Payloads removed"} · {size(record.entry.payloadBytes)} of lesson
          data{record.editorialStatus === "draft" ? " · Unreviewed draft" : ""}
        </p>
        {#if record.error}<p role="alert">
            {record.error} Choose the original folder to repair this version.
          </p>{/if}
        <div class="actions">
          {#if record.entry.stored && !record.error && !record.entry.active}<button
              class="secondary"
              disabled={busy || !!ticket || !!decision}
              onclick={() => ask("activate", record)}
              >Use version {record.entry.version}</button
            >{/if}
          {#if record.entry.active}<button
              class="secondary"
              disabled={busy || !!ticket || !!decision}
              onclick={() => ask("deactivate", record)}
              >Deactivate version {record.entry.version}</button
            >{:else if record.entry.stored && !record.error}<button
              class="secondary"
              disabled={busy || !!ticket || !!decision}
              onclick={() => ask("remove", record)}
              >Remove version {record.entry.version}</button
            >{/if}
        </div>
      </li>
    {/each}
  </ul>
</section>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.7rem;
    margin-block: 1rem;
  }
  .packs {
    list-style: none;
    padding: 0;
  }
  .packs li {
    padding-block: 1rem;
    border-top: 1px solid var(--border);
    overflow-wrap: anywhere;
  }
  .packs p {
    margin-block: 0.5rem;
  }
  .callout {
    margin-block: 1rem;
    overflow-wrap: anywhere;
  }
</style>
