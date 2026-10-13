<script lang="ts">
  import {
    command,
    commandTooltip,
    disabledReason,
    runCommand,
    type CommandId,
  } from "$lib/commands.svelte";
  import type { Icon } from "$lib/icons";

  /**
   * A button for one command of the registry (T3.15): label, icon, tooltip and disabled state
   * all come from `commands.svelte.ts`. `large` is the big ribbon button (28 px icon over the
   * label, at least 40 px high), `small` an icon with the label beside it (32 px), `icon` an
   * icon only button for universally known actions (decision 26), named by its label.
   *
   * Disabled commands stay visible and focusable (`aria-disabled`), so the tooltip with the
   * reason still shows on hover and is read by screen readers (decision 27).
   */
  interface Props {
    id: CommandId;
    size?: "large" | "small" | "icon";
    /** Replaces the registry icon, for example the sun or moon of the theme switch. */
    icon?: Icon;
  }

  let { id, size = "small", icon }: Props = $props();

  const def = $derived(command(id));
  const reason = $derived(disabledReason(id));
  const tooltip = $derived(commandTooltip(id));
  const pressed = $derived(def.pressed?.());
  const Glyph = $derived(icon ?? def.icon);

  const classes = {
    large:
      "flex min-h-10 min-w-[4.5rem] max-w-28 flex-col items-center gap-1 rounded-lg border border-transparent px-1.5 py-1.5 text-center text-[13px] leading-tight",
    small:
      "flex min-h-8 items-center gap-1.5 rounded-md border border-transparent px-2 py-0.5 text-left text-[13px] whitespace-nowrap",
    icon: "flex size-9 items-center justify-center rounded-lg border border-transparent",
  } as const;
</script>

<button
  type="button"
  class="cmd {classes[
    size
  ]} text-text hover:bg-hover aria-disabled:cursor-default aria-disabled:opacity-45 aria-disabled:hover:bg-transparent aria-pressed:border-accent aria-pressed:bg-accent-soft"
  title={tooltip}
  aria-label={size === "icon" ? def.label() : undefined}
  aria-disabled={reason !== null ? "true" : undefined}
  aria-pressed={pressed}
  data-command={id}
  onclick={() => {
    runCommand(id);
  }}
>
  <Glyph
    size={size === "large" ? 28 : size === "icon" ? 20 : 18}
    strokeWidth={1.75}
    aria-hidden="true"
    class="shrink-0 {pressed ? 'text-accent-strong' : 'text-text-muted'}"
  />
  {#if size !== "icon"}
    <span>{def.label()}</span>
  {/if}
</button>
