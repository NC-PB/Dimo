<script lang="ts">
  import { COLORS, OUTLINES_MM, SHAPES, SIZES_MM, shapeLabel } from "$lib/balloon-style-options";
  import ShapeIcon from "$lib/components/ShapeIcon.svelte";
  import { LOCALE_NAMES, commandErrorMessage, locales, m, themeLabel } from "$lib/i18n";
  import type { BalloonStyle, Theme } from "$lib/ipc/bindings";
  import { baseLocale, extractLocaleFromNavigator } from "$lib/paraglide/runtime.js";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";

  const THEMES: readonly Theme[] = ["system", "light", "dark"];
  /** Select value for "follow the system language". */
  const SYSTEM = "system";
  /** The language the operating system asks for, as the app would use it. */
  const systemLocale = extractLocaleFromNavigator() ?? baseLocale;

  const settings = $derived(settingsStore.current);
  const style = $derived(projectStore.project?.settings.balloon_style ?? null);
  /** The language shown as chosen: the stored one, or "system" while none is stored. */
  const chosenLocale = $derived(settings.locale ?? SYSTEM);

  /** Sets the project default balloon style (an undoable command, FR-BAL-03). */
  function setStyle(change: Partial<BalloonStyle>): void {
    if (style === null) {
      return;
    }
    void projectStore.execute({
      type: "set_default_balloon_style",
      style: { ...style, ...change },
    });
  }

  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-text";
  const optionClass =
    "flex items-center gap-1 rounded border border-border px-2 py-1 text-xs text-text hover:bg-surface-raised aria-pressed:border-accent aria-pressed:ring-2 aria-pressed:ring-accent";
  const sectionClass = "flex max-w-2xl flex-col gap-3 rounded border border-border bg-surface p-4";
</script>

<section
  class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto bg-bg p-6 text-sm text-text"
  aria-labelledby="settings-heading"
>
  <h1 id="settings-heading" class="text-lg font-semibold">{m.view_settings()}</h1>

  {#if settingsStore.error}
    <p class="max-w-2xl text-danger" role="alert">
      <span aria-hidden="true">⚠</span>
      {commandErrorMessage(settingsStore.error)}
    </p>
  {/if}

  <fieldset class={sectionClass}>
    <legend class="px-1 font-semibold">{m.settings_appearance()}</legend>
    <label class="flex flex-wrap items-center gap-2">
      <span class="w-48">{m.theme_label()}</span>
      <select
        class={fieldClass}
        value={settings.theme}
        onchange={(e) => {
          const theme = THEMES.find((t) => t === e.currentTarget.value);
          if (theme) {
            void settingsStore.update({ theme });
          }
        }}
      >
        {#each THEMES as theme (theme)}
          <option value={theme}>{themeLabel(theme)}</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-wrap items-center gap-2">
      <span class="w-48">{m.language_label()}</span>
      <select
        class={fieldClass}
        aria-describedby="language-hint"
        value={chosenLocale}
        onchange={(e) => {
          const value = e.currentTarget.value;
          void settingsStore.setLocale(value === SYSTEM ? null : value);
        }}
      >
        <option value={SYSTEM}>{m.language_system()} ({LOCALE_NAMES[systemLocale]})</option>
        {#each locales as l (l)}
          <option value={l}>{LOCALE_NAMES[l]}</option>
        {/each}
      </select>
      <span id="language-hint" class="text-xs text-text-muted">{m.language_hint()}</span>
    </label>
  </fieldset>

  <fieldset class={sectionClass}>
    <legend class="px-1 font-semibold">{m.settings_audit()}</legend>
    <label class="flex flex-wrap items-center gap-2">
      <span class="w-48">{m.user_name_label()}</span>
      <input
        class="{fieldClass} w-64"
        type="text"
        autocomplete="off"
        spellcheck="false"
        aria-describedby="user-name-hint"
        placeholder={settingsStore.osUserName}
        value={settings.user_name}
        onchange={(e) => void settingsStore.update({ user_name: e.currentTarget.value })}
      />
    </label>
    <p id="user-name-hint" class="text-xs text-text-muted">
      {m.user_name_hint({ name: settingsStore.osUserName })}
    </p>
  </fieldset>

  <fieldset class={sectionClass}>
    <legend class="px-1 font-semibold">{m.settings_balloon_defaults()}</legend>
    {#if style === null}
      <p class="text-text-muted">{m.settings_no_project()}</p>
    {:else}
      <p class="text-xs text-text-muted">{m.settings_balloon_defaults_hint()}</p>
      <fieldset class="flex flex-wrap items-center gap-2">
        <legend class="float-left w-48">{m.balloon_shape()}</legend>
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
        <span class="w-48">{m.balloon_size()}</span>
        <select
          class={fieldClass}
          value={String(style.size_mm)}
          onchange={(e) => {
            setStyle({ size_mm: Number(e.currentTarget.value) });
          }}
        >
          {#if style.size_mm !== null && !SIZES_MM.includes(style.size_mm)}
            <option value={String(style.size_mm)}
              >{m.size_mm({ size: String(style.size_mm) })}</option
            >
          {/if}
          {#each SIZES_MM as size (size)}
            <option value={String(size)}>{m.size_mm({ size: String(size) })}</option>
          {/each}
        </select>
      </label>
      <label class="flex flex-wrap items-center gap-2">
        <span class="w-48">{m.balloon_outline()}</span>
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
        <legend class="float-left w-48">{m.balloon_color()}</legend>
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
  </fieldset>
</section>
