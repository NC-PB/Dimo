<script lang="ts" module>
  import type { CommandId } from "$lib/commands.svelte";
  import type { Tab } from "$lib/stores/view.svelte";

  /**
   * A button of an empty state: a command of the registry, or a tab to switch to (navigation
   * is view state, not a command). The first action is the primary one.
   */
  export type EmptyAction = { command: CommandId } | { tab: Tab; label: string };
</script>

<script lang="ts">
  import { command, commandTooltip, disabledReason, runCommand } from "$lib/commands.svelte";
  import type { Icon } from "$lib/icons";
  import { view } from "$lib/stores/view.svelte";

  /**
   * Empty state of a view or pane (T3.17): what is (not) here, why, and a button
   * for the next step. `page` fills a work area, `inline` sits inside a pane or above content.
   */
  interface Props {
    title: string;
    text: string;
    icon?: Icon;
    actions?: readonly EmptyAction[];
    size?: "page" | "inline";
  }

  let { title, text, icon, actions = [], size = "page" }: Props = $props();

  const Glyph = $derived(icon);

  function label(action: EmptyAction): string {
    return "command" in action ? command(action.command).label() : action.label;
  }

  function run(action: EmptyAction): void {
    if ("command" in action) {
      runCommand(action.command);
    } else {
      view.setTab(action.tab);
    }
  }

  const base =
    "inline-flex min-h-10 items-center rounded-lg px-4 text-sm aria-disabled:cursor-default aria-disabled:opacity-45";
  // A command that is on (Place balloon while placing, a filter) looks like a pressed button.
  const pressedClass =
    "aria-pressed:border-2 aria-pressed:border-accent aria-pressed:bg-accent-soft aria-pressed:text-text";
  const primary = `${base} bg-accent-strong font-semibold text-accent-ink hover:opacity-90 ${pressedClass}`;
  const secondary = `${base} border border-border-strong bg-surface text-text hover:bg-hover ${pressedClass}`;
</script>

<div
  class={size === "page"
    ? "m-auto flex max-w-md flex-col items-center gap-3 p-6 text-center"
    : "flex flex-col items-start gap-2 p-3"}
  data-testid="empty-state"
>
  {#if Glyph}
    <span
      class="flex items-center justify-center rounded-xl bg-accent-soft text-accent-strong {size ===
      'page'
        ? 'size-12'
        : 'size-9'}"
      aria-hidden="true"
    >
      <Glyph size={size === "page" ? 26 : 18} strokeWidth={1.75} />
    </span>
  {/if}
  <h2
    class={size === "page"
      ? "font-heading text-lg font-semibold text-text"
      : "text-sm font-semibold text-text"}
  >
    {title}
  </h2>
  <p class="text-sm leading-relaxed text-text-muted">{text}</p>
  {#if actions.length > 0}
    <div class="flex flex-wrap gap-2 {size === 'page' ? 'justify-center' : ''}">
      {#each actions as action, index ("command" in action ? action.command : action.tab)}
        {@const reason = "command" in action ? disabledReason(action.command) : null}
        <button
          type="button"
          class={index === 0 ? primary : secondary}
          title={"command" in action ? commandTooltip(action.command) : undefined}
          aria-disabled={reason !== null ? "true" : undefined}
          aria-pressed={"command" in action ? command(action.command).pressed?.() : undefined}
          data-action={"command" in action ? action.command : `tab-${action.tab}`}
          onclick={() => {
            run(action);
          }}
        >
          {label(action)}
        </button>
      {/each}
    </div>
  {/if}
</div>
