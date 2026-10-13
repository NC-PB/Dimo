<script lang="ts">
  /**
   * File > About (M3 decision 29, T3.16): the Dimo version, its Apache 2.0 license, the NOTICE
   * file, and for every bundled font and icon set the copyright notice with the full license
   * text. The texts are embedded in the app (`about.ts`), so they show without network. License
   * texts are legal documents and stay in their original English.
   */
  import { BUNDLED, DIMO_LICENSE, DIMO_NOTICE } from "$lib/about";
  import { buildProfileLabel, m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import { appInfo } from "$lib/stores/app-info.svelte";

  const info = $derived(appInfo.current);

  let copied = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy(id: string, text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      return; // The clipboard may be unavailable; the text can still be selected by hand.
    }
    copied = id;
    clearTimeout(timer);
    timer = setTimeout(() => {
      copied = null;
    }, 1500);
  }

  const boxClass = "flex flex-col gap-2 rounded-xl border border-border bg-surface p-4 shadow-sm";
  const textClass =
    "max-h-64 overflow-auto rounded-md border border-border bg-sunken p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-text";
  const CopyIcon = icons.copy;
</script>

{#snippet licenseText(id: string, label: string, text: string)}
  <div class="flex items-center justify-between gap-2">
    <span class="text-xs text-text-muted">{label}</span>
    <button
      type="button"
      class="flex min-h-8 items-center gap-1 rounded-md border border-border px-2 py-0.5 text-xs hover:bg-hover"
      aria-label={m.about_copy_aria({ name: label })}
      onclick={() => void copy(id, text)}
    >
      <CopyIcon size={14} strokeWidth={1.75} aria-hidden="true" class="text-text-muted" />
      {copied === id ? m.copy_hash_done() : m.about_copy()}
    </button>
  </div>
  <!-- Focusable so the text scrolls with the keyboard. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <pre class={textClass} tabindex="0" aria-label={label} data-license={id}>{text}</pre>
{/snippet}

<div class="flex max-w-3xl flex-col gap-5" data-testid="about-pane">
  <div>
    <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.about_title()}</h1>
    <p class="mt-1 text-text-muted">{m.about_lead()}</p>
  </div>

  <section class={boxClass} aria-labelledby="about-dimo">
    <h2 id="about-dimo" class="font-heading text-base font-semibold">Dimo</h2>
    {#if info}
      <p data-testid="about-version">
        {m.app_version({ version: info.version })}, {buildProfileLabel(info.build_profile)}
      </p>
    {/if}
    <p class="text-text-muted">{m.about_license()}</p>
    {@render licenseText("dimo", "Apache License 2.0", DIMO_LICENSE)}
    {@render licenseText("notice", "NOTICE", DIMO_NOTICE)}
  </section>

  <section class="flex flex-col gap-3" aria-labelledby="about-bundled">
    <div>
      <h2 id="about-bundled" class="font-heading text-xl font-semibold">{m.about_bundled()}</h2>
      <p class="mt-1 text-text-muted">{m.about_bundled_lead()}</p>
    </div>
    {#each BUNDLED as asset (asset.id)}
      <article class={boxClass} aria-labelledby="about-{asset.id}">
        <h3 id="about-{asset.id}" class="font-heading font-semibold">{asset.name}</h3>
        <dl class="grid grid-cols-[minmax(6rem,10rem)_minmax(0,1fr)] gap-x-3 gap-y-1 text-sm">
          <dt class="text-text-muted">{m.about_copyright()}</dt>
          <dd>{asset.copyright}</dd>
          <dt class="text-text-muted">{m.about_license_name()}</dt>
          <dd>{asset.license}</dd>
          {#if asset.files.length > 0}
            <dt class="text-text-muted">{m.about_files()}</dt>
            <dd class="font-mono text-xs break-all">{asset.files.join(", ")}</dd>
          {/if}
        </dl>
        {@render licenseText(asset.id, asset.license, asset.text)}
      </article>
    {/each}
  </section>

  <section class={boxClass} aria-labelledby="about-third-party">
    <h2 id="about-third-party" class="font-heading text-base font-semibold">
      {m.about_third_party()}
    </h2>
    <p class="text-text-muted">{m.about_third_party_text()}</p>
  </section>
</div>
