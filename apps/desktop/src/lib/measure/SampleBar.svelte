<script lang="ts">
  import { m } from "$lib/i18n";
  import type { Command, CommandError, SampleId } from "$lib/ipc/bindings";
  import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
  import { refusalDetail } from "$lib/table/refusal";

  interface Props {
    /** The project; the app's store unless a test passes its own. */
    project?: ProjectStore;
    /** Sample of the active grid column; move and delete act on it. */
    activeSample: SampleId | null;
  }

  let { project = projectStore, activeSample }: Props = $props();

  /**
   * Sample bar of the Measure view (T3.5, FR-MEA-01, M3 decision 6): add N samples, move the
   * active sample left or right, delete it. Deleting a sample with results asks first, since
   * its results go with it (decision 16). Every action is one undoable Rust command.
   */
  let count = $state(1);
  let notice = $state<string | null>(null);
  /** The sample waiting for "delete with its results" to be confirmed. */
  let confirming = $state<SampleId | null>(null);

  const samples = $derived(project.project?.samples ?? []);
  const index = $derived(
    activeSample === null ? -1 : samples.findIndex((s) => s.id === activeSample),
  );
  const sample = $derived(index >= 0 ? samples[index] : undefined);
  const resultCount = (id: SampleId) =>
    (project.project?.results ?? []).filter((r) => r.sample === id).length;

  async function send(command: Command): Promise<boolean> {
    const patch = await project.execute(command);
    if (patch !== undefined) {
      notice = null;
      return true;
    }
    const error: CommandError | null = project.error;
    project.dismissError();
    notice = error === null ? null : refusalDetail(error);
    return false;
  }

  function add() {
    // Rust checks the count (1 to 100) and the limit of 500 samples per project.
    const n = Number.isInteger(count) ? count : Number.NaN;
    if (Number.isNaN(n)) {
      notice = m.refusal_invalid_sample_count();
      return;
    }
    void send({ type: "add_samples", count: n });
  }

  function moveBy(step: -1 | 1) {
    if (sample === undefined) {
      return;
    }
    // Insert before the sample at the target place; moving right skips the next one.
    const before = step < 0 ? samples[index - 1]?.id : samples[index + 2]?.id;
    if (step < 0 && before === undefined) {
      return;
    }
    if (step > 0 && index >= samples.length - 1) {
      return;
    }
    void send({ type: "move_samples", ids: [sample.id], before: before ?? null });
  }

  function remove() {
    if (sample === undefined) {
      return;
    }
    if (resultCount(sample.id) > 0 && confirming !== sample.id) {
      confirming = sample.id;
      return;
    }
    const id = sample.id;
    confirming = null;
    void send({ type: "delete_samples", ids: [id] });
  }
</script>

<div
  class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border bg-surface px-3 py-1.5 text-sm"
  role="toolbar"
  aria-label={m.samples_bar_label()}
>
  <label class="flex items-center gap-1 text-xs text-text-muted">
    {m.samples_add_count()}
    <input
      type="number"
      min="1"
      max="100"
      class="h-7 w-14 rounded border border-border bg-surface px-1 text-sm text-text"
      bind:value={count}
      onkeydown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          add();
        }
      }}
    />
  </label>
  <button class="h-7 rounded bg-accent-strong px-3 text-sm text-accent-ink" onclick={add}
    >{m.samples_add()}</button
  >
  <span class="mx-1 h-5 w-px bg-border" aria-hidden="true"></span>
  {#if sample}
    <span class="text-xs text-text-muted">{m.samples_active({ serial: sample.serial })}</span>
    <button
      class="h-7 rounded border border-border px-2 text-sm disabled:opacity-40"
      disabled={index <= 0}
      onclick={() => {
        moveBy(-1);
      }}>{m.samples_move_left()}</button
    >
    <button
      class="h-7 rounded border border-border px-2 text-sm disabled:opacity-40"
      disabled={index >= samples.length - 1}
      onclick={() => {
        moveBy(1);
      }}>{m.samples_move_right()}</button
    >
    {#if confirming === sample.id}
      <span class="text-xs text-danger" role="alert"
        >{m.samples_delete_confirm({
          serial: sample.serial,
          count: String(resultCount(sample.id)),
        })}</span
      >
      <button class="h-7 rounded bg-danger px-2 text-sm text-accent-ink" onclick={remove}
        >{m.samples_delete_yes()}</button
      >
      <button
        class="h-7 rounded border border-border px-2 text-sm"
        onclick={() => {
          confirming = null;
        }}>{m.samples_delete_no()}</button
      >
    {:else}
      <button class="h-7 rounded border border-border px-2 text-sm text-danger" onclick={remove}
        >{m.samples_delete()}</button
      >
    {/if}
  {:else}
    <span class="text-xs text-text-muted">{m.samples_none_active()}</span>
  {/if}
  {#if notice !== null}
    <span class="text-xs text-danger" role="alert" data-testid="sample-notice">{notice}</span>
  {/if}
</div>
