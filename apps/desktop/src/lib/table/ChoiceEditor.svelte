<script lang="ts">
  import { Select } from "bits-ui";
  import type { ChoiceOption } from "./columns";

  interface Props {
    /** Current value. */
    value: string;
    options: ChoiceOption[];
    /** Accessible name, the column header. */
    label: string;
    /** A value was picked. */
    onPick: (value: string) => void;
    /** The list closed, with or without a pick. */
    onClose: () => void;
  }

  let { value, options, label, onPick, onClose }: Props = $props();

  const current = $derived(options.find((o) => o.value === value)?.label ?? "");
</script>

<!-- Opened by Enter or a double click on a kind, unit or classification cell (Bits UI). -->
<Select.Root
  type="single"
  open={true}
  {value}
  items={options}
  onOpenChange={(open) => {
    if (!open) {
      onClose();
    }
  }}
  onValueChange={(picked) => {
    if (picked !== value) {
      onPick(picked);
    }
  }}
>
  <Select.Trigger
    class="h-full w-full truncate bg-surface px-2 text-left text-text outline-2 -outline-offset-2 outline-focus"
    aria-label={label}
  >
    {current}
  </Select.Trigger>
  <Select.Portal>
    <Select.Content
      class="z-50 max-h-72 min-w-36 overflow-auto rounded border border-border bg-surface p-1 text-sm text-text shadow-lg"
      sideOffset={2}
    >
      <Select.Viewport>
        {#each options as option (option.value)}
          <Select.Item
            value={option.value}
            label={option.label}
            class="flex cursor-default items-center gap-2 rounded px-2 py-1 data-highlighted:bg-surface-raised"
          >
            {#snippet children({ selected })}
              <span class="w-3 text-center" aria-hidden="true">{selected ? "✓" : ""}</span>
              {option.label}
            {/snippet}
          </Select.Item>
        {/each}
      </Select.Viewport>
    </Select.Content>
  </Select.Portal>
</Select.Root>
