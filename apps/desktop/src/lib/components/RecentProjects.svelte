<script lang="ts">
  /**
   * The recent projects list (T3.16, ADR 0007): one click opens a project, files that are gone
   * are shown greyed with the reason and can be removed from the list. Rust keeps the list and
   * opens only paths that are on it. Reusable: File > Open shows the full list, the start screen
   * can show a compact one (`compact`, `limit`).
   */
  import { m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import { recentStore as appRecent, type RecentStore } from "$lib/stores/recent.svelte";
  import { untrack } from "svelte";

  interface Props {
    /** Smaller rows without the folder line wrapping, for the start screen. */
    compact?: boolean;
    /** Show at most this many entries (all by default). */
    limit?: number;
    /** Heading above the list; none when empty. */
    heading?: string;
    store?: RecentStore;
    project?: ProjectStore;
  }

  let {
    compact = false,
    limit,
    heading,
    store = appRecent,
    project = appProject,
  }: Props = $props();

  // The list changes when a project is opened or saved under a new name: read it again then.
  $effect(() => {
    void project.session;
    void project.status?.file_name;
    untrack(() => void store.load());
  });

  const items = $derived(limit === undefined ? store.items : store.items.slice(0, limit));
  const FileIcon = icons.recentFile;
  const RemoveIcon = icons.remove;
  const uid = $props.id();
</script>

<section class="flex flex-col gap-2" aria-labelledby={heading ? `${uid}-recent` : undefined}>
  {#if heading}
    <h2 id="{uid}-recent" class="font-heading text-base font-semibold">{heading}</h2>
  {/if}
  {#if items.length === 0}
    <p class="text-sm text-text-muted" data-testid="recent-empty">{m.recent_empty()}</p>
  {:else}
    <ul class="flex flex-col gap-1" aria-label={heading ?? m.recent_title()}>
      {#each items as item (item.path)}
        <li class="group flex items-center gap-1 rounded-lg hover:bg-hover" data-recent={item.path}>
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-3 rounded-lg px-3 text-left {compact
              ? 'min-h-10 py-1'
              : 'min-h-12 py-2'} aria-disabled:cursor-default"
            title={item.exists ? item.path : m.recent_missing_title({ path: item.path })}
            aria-disabled={item.exists ? undefined : "true"}
            disabled={project.busy}
            onclick={() => {
              if (item.exists) {
                void store.open(item.path);
              }
            }}
          >
            <FileIcon
              size={compact ? 18 : 22}
              strokeWidth={1.75}
              aria-hidden="true"
              class="shrink-0 text-text-muted"
            />
            <span class="flex min-w-0 flex-col">
              <span
                class="truncate font-semibold {item.exists
                  ? 'text-text'
                  : 'text-text-muted line-through'}">{item.name}</span
              >
              <span class="truncate font-mono text-xs text-text-muted">
                {#if !item.exists}
                  <span class="font-sans">{m.recent_missing()}:</span>
                {/if}
                {item.folder}
              </span>
            </span>
          </button>
          <button
            type="button"
            class="flex size-8 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-surface-raised hover:text-text {item.exists
              ? 'opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 focus:opacity-100'
              : ''}"
            title={m.recent_remove({ name: item.name })}
            aria-label={m.recent_remove({ name: item.name })}
            onclick={() => void store.remove(item.path)}
          >
            <RemoveIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
