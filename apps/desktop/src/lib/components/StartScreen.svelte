<script lang="ts">
  import { commandTooltip, disabledReason, runCommand, type CommandId } from "$lib/commands.svelte";
  import { m } from "$lib/i18n";
  import { icons, type Icon } from "$lib/icons";
  import RecentProjects from "./RecentProjects.svelte";

  /**
   * Start screen while no project is open (T3.17, mockup "Start"): a greeting, the two ways in
   * (new project from a drawing, open a project), the last five recent projects (T3.16's
   * `RecentProjects`, compact) and how the work goes in three steps. The cards run the registry
   * commands, so the File page, the keys and this screen share one disabled state.
   */
  interface Props {
    /** The time the greeting is for; tests pass a fixed one. */
    now?: Date;
  }

  let { now = new Date() }: Props = $props();

  const greeting = $derived.by(() => {
    const hour = now.getHours();
    if (hour < 12) {
      return m.start_greeting_morning();
    }
    return hour < 18 ? m.start_greeting_afternoon() : m.start_greeting_evening();
  });

  interface Card {
    id: CommandId;
    title: () => string;
    text: () => string;
    icon: Icon;
    primary: boolean;
  }

  const CARDS: readonly Card[] = [
    {
      id: "new_project",
      title: () => m.start_new_title(),
      text: () => m.start_new_text(),
      icon: icons.newProject,
      primary: true,
    },
    {
      id: "open_project",
      title: () => m.start_open_title(),
      text: () => m.start_open_text(),
      icon: icons.openProject,
      primary: false,
    },
  ];

  const STEPS = [
    { title: () => m.start_step1_title(), text: () => m.start_step1_text() },
    { title: () => m.start_step2_title(), text: () => m.start_step2_text() },
    { title: () => m.start_step3_title(), text: () => m.start_step3_text() },
  ] as const;
</script>

<section
  class="flex min-h-0 flex-1 justify-center overflow-y-auto bg-bg text-text"
  aria-label={m.start_label()}
  data-testid="start-screen"
>
  <div class="flex w-full max-w-[1100px] flex-col gap-8 px-6 py-10 sm:px-14">
    <div>
      <h1 class="font-heading text-[34px] font-semibold tracking-tight">{greeting}</h1>
      <p class="mt-2 max-w-[640px] text-base leading-normal text-text-muted">{m.start_lead()}</p>
    </div>

    <div class="flex flex-wrap gap-4">
      {#each CARDS as card (card.id)}
        {@const reason = disabledReason(card.id)}
        <button
          type="button"
          class="flex min-h-24 flex-[1_1_260px] items-start gap-4 rounded-2xl border bg-surface p-5 text-left shadow-sm hover:border-accent aria-disabled:cursor-default aria-disabled:opacity-60 {card.primary
            ? 'border-2 border-accent'
            : 'border-border'}"
          title={commandTooltip(card.id)}
          aria-labelledby="start-{card.id}-title"
          aria-describedby="start-{card.id}-text"
          aria-disabled={reason !== null ? "true" : undefined}
          data-action={card.id}
          onclick={() => {
            runCommand(card.id);
          }}
        >
          <span
            class="flex size-12 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-strong"
            aria-hidden="true"
          >
            <card.icon size={26} strokeWidth={1.7} />
          </span>
          <span class="flex flex-col gap-1">
            <span id="start-{card.id}-title" class="font-heading text-lg font-semibold">
              {card.title()}
            </span>
            <span id="start-{card.id}-text" class="text-[13px] leading-normal text-text-muted">
              {card.text()}
            </span>
          </span>
        </button>
      {/each}
    </div>

    <div data-slot="recent-projects">
      <RecentProjects compact limit={5} heading={m.recent_title()} />
    </div>

    <section aria-labelledby="start-steps-title">
      <h2
        id="start-steps-title"
        class="mb-3 text-xs font-medium tracking-[0.08em] text-text-muted uppercase"
      >
        {m.start_steps_title()}
      </h2>
      <ol class="flex flex-wrap gap-3">
        {#each STEPS as step, index (index)}
          <li class="flex flex-[1_1_200px] items-start gap-3 text-[13px] leading-normal">
            <span
              class="flex size-7 shrink-0 items-center justify-center rounded-full border-2 border-accent-strong font-heading font-bold text-accent-strong"
              aria-hidden="true">{index + 1}</span
            >
            <span class="text-text-muted">
              <strong class="block text-sm text-text">{step.title()}</strong>
              {step.text()}
            </span>
          </li>
        {/each}
      </ol>
    </section>
  </div>
</section>
