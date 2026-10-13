<script lang="ts">
  /**
   * General tolerance on the Home and Drawing tabs (M3 decision 21, T3.15): one select with
   * every class of every general table, bound to the project's tolerance settings. A change is
   * the same undoable `set_tolerance_settings` as on the settings page, and offers the update of
   * existing characteristics (decision 22, shown above the drawing). Rust lists the tables.
   */
  import { m } from "$lib/i18n";
  import type { TableClass } from "$lib/ipc/bindings";
  import { projectStore } from "$lib/stores/project.svelte";
  import { toleranceStore } from "$lib/stores/tolerance.svelte";
  import { FIELD_CLASS, LABEL_CLASS, fieldTitle } from "./field";

  const NONE = "";
  const settings = $derived(toleranceStore.settings);

  // The table list follows the project: an import, undo or another project.
  $effect(() => {
    void projectStore.session;
    void projectStore.project?.settings.tolerance;
    void toleranceStore.refreshTables();
  });

  const options = $derived(
    toleranceStore.generalTables.flatMap((t) =>
      t.classes.map((cls) => ({
        value: `${t.table.id}|${cls}`,
        label: `${t.title} ${cls}`,
        choice: { table: t.table, class: cls } satisfies TableClass,
        draft: t.draft,
      })),
    ),
  );
  const general = $derived(settings?.general ?? null);
  const value = $derived(general === null ? NONE : `${general.table.id}|${general.class}`);
  const draft = $derived(options.find((o) => o.value === value)?.draft ?? false);

  function onChange(event: Event & { currentTarget: HTMLSelectElement }): void {
    const picked = event.currentTarget.value;
    const choice = picked === NONE ? null : options.find((o) => o.value === picked)?.choice;
    if (choice !== undefined) {
      void toleranceStore.setSettings({ general: choice });
    }
  }
</script>

<div class="flex flex-col gap-1">
  <label
    class={LABEL_CLASS}
    title={fieldTitle(
      m.field_general_tolerance_desc(),
      settings === null ? m.reason_no_project() : null,
    )}
  >
    {m.tolerance_general()}
    <select
      class="{FIELD_CLASS} min-w-48"
      data-field="general-tolerance"
      disabled={settings === null}
      {value}
      onchange={onChange}
    >
      <option value={NONE}>{m.tolerance_none()}</option>
      {#each options as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
      {#if general !== null && !options.some((o) => o.value === value)}
        <!-- The stored choice until Rust's table list arrives. -->
        <option {value}>{general.table.id} {general.class}</option>
      {/if}
    </select>
  </label>
  {#if draft}
    <span
      class="self-start rounded-full border border-dashed border-warning px-2 text-[11px] font-semibold text-warning"
      title={m.badge_draft_title()}>{m.badge_draft()}</span
    >
  {/if}
</div>
