<script lang="ts">
  /**
   * The balloon style of the open project (FR-BAL-03): shape, size, line width, outline color
   * and leader line of every balloon without a style of its own. Shown in a dialog of the
   * Drawing tab (M3 decision 21, T3.16). Each change is one undoable
   * `set_default_balloon_style` command.
   */
  import { COLORS, OUTLINES_MM, SHAPES, SIZES_MM, shapeLabel } from "$lib/balloon-style-options";
  import ShapeIcon from "$lib/components/ShapeIcon.svelte";
  import { m } from "$lib/i18n";
  import type { BalloonStyle } from "$lib/ipc/bindings";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";

  interface Props {
    project?: ProjectStore;
  }

  let { project = appProject }: Props = $props();

  const style = $derived(project.project?.settings.balloon_style ?? null);

  function setStyle(change: Partial<BalloonStyle>): void {
    if (style === null) {
      return;
    }
    void project.execute({ type: "set_default_balloon_style", style: { ...style, ...change } });
  }

  const fieldClass = "min-h-8 rounded border border-border-strong bg-surface px-2 py-1 text-text";
  const optionClass =
    "flex min-h-8 items-center gap-1 rounded border border-border px-2 py-1 text-xs text-text hover:bg-hover aria-pressed:border-accent aria-pressed:ring-2 aria-pressed:ring-accent";
</script>

<div class="flex flex-col gap-3 text-sm text-text" data-testid="project-balloon-style">
  {#if style === null}
    <p class="text-text-muted">{m.settings_no_project()}</p>
  {:else}
    <p class="text-xs text-text-muted">{m.settings_balloon_defaults_hint()}</p>
    <fieldset class="flex flex-wrap items-center gap-2">
      <legend class="float-left w-32">{m.balloon_shape()}</legend>
      {#each SHAPES as shape (shape)}
        <button
          type="button"
          class={optionClass}
          aria-pressed={style.shape === shape}
          onclick={() => {
            setStyle({ shape });
          }}
        >
          <ShapeIcon {shape} />
          {shapeLabel(shape)}
        </button>
      {/each}
    </fieldset>
    <label class="flex flex-wrap items-center gap-2">
      <span class="w-32">{m.balloon_size()}</span>
      <select
        class={fieldClass}
        value={String(style.size_mm)}
        onchange={(e) => {
          setStyle({ size_mm: Number(e.currentTarget.value) });
        }}
      >
        {#if style.size_mm !== null && !SIZES_MM.includes(style.size_mm)}
          <option value={String(style.size_mm)}>{m.size_mm({ size: String(style.size_mm) })}</option
          >
        {/if}
        {#each SIZES_MM as size (size)}
          <option value={String(size)}>{m.size_mm({ size: String(size) })}</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-wrap items-center gap-2">
      <span class="w-32">{m.balloon_outline()}</span>
      <select
        class={fieldClass}
        value={String(style.outline_mm)}
        onchange={(e) => {
          setStyle({ outline_mm: Number(e.currentTarget.value) });
        }}
      >
        {#if style.outline_mm !== null && !OUTLINES_MM.includes(style.outline_mm)}
          <option value={String(style.outline_mm)}>
            {m.size_mm({ size: String(style.outline_mm) })}
          </option>
        {/if}
        {#each OUTLINES_MM as width (width)}
          <option value={String(width)}>{m.size_mm({ size: String(width) })}</option>
        {/each}
      </select>
    </label>
    <fieldset class="flex flex-wrap items-center gap-2">
      <legend class="float-left w-32">{m.balloon_color()}</legend>
      {#each COLORS as option (option.color)}
        <button
          type="button"
          class={optionClass}
          aria-pressed={style.outline_color === option.color}
          onclick={() => {
            setStyle({ outline_color: option.color });
          }}
        >
          <span
            class="inline-block h-3 w-3 rounded-full border border-border"
            style:background={option.color}
            aria-hidden="true"
          ></span>
          {option.label()}
        </button>
      {/each}
    </fieldset>
    <label class="flex items-center gap-2">
      <input
        type="checkbox"
        checked={style.leader}
        onchange={(e) => {
          setStyle({ leader: e.currentTarget.checked });
        }}
      />
      {m.balloon_leader()}
    </label>
  {/if}
</div>
