/**
 * License texts shown in File > About (M3 decision 29, T3.16). Every text is embedded into the
 * app bundle at build time (Vite `?raw`), so it shows without network and travels with every
 * copy, as the SIL OFL 1.1 requires for the bundled fonts. The texts are read unchanged from
 * their files: `LICENSE` and `NOTICE` of the repository, the OFL files next to the fonts in
 * `data/fonts/`, and the `LICENSE` file of the installed Lucide package. Never edit them here.
 *
 * `about.test.ts` checks that every font file in `data/fonts/` belongs to an entry below and
 * that the Lucide license is included.
 */

import apache from "../../../../LICENSE?raw";
import notice from "../../../../NOTICE?raw";
import jetBrainsMonoOfl from "../../../../data/fonts/JetBrainsMono-OFL.txt?raw";
import spaceGroteskOfl from "../../../../data/fonts/SpaceGrotesk-OFL.txt?raw";
import lucide from "../../node_modules/@lucide/svelte/LICENSE?raw";

/** One bundled component with its copyright notice and full license text. */
export interface LicensedAsset {
  /** Stable key, also the element ID suffix. */
  id: string;
  /** Name of the component, not translated. */
  name: string;
  /** Copyright notice as the component states it, not translated. */
  copyright: string;
  /** License name, not translated. */
  license: string;
  /** Font files in `data/fonts/` covered by this entry (empty for other assets). */
  files: readonly string[];
  /** The full license text, unchanged. */
  text: string;
}

/** The Apache License 2.0 of DimoMark itself (`LICENSE`). */
export const DIMO_LICENSE = apache;

/** The `NOTICE` file: copyright of DimoMark and the notices of bundled third party components. */
export const DIMO_NOTICE = notice;

/** Bundled fonts and assets, in the order About lists them. */
export const BUNDLED: readonly LicensedAsset[] = [
  {
    id: "open-sans",
    name: "Open Sans 1.10 (Regular, Bold)",
    copyright: "Digitized data copyright 2010-2011 Google Corporation",
    license: "Apache License 2.0",
    files: ["OpenSans-Regular.ttf", "OpenSans-Bold.ttf"],
    text: apache,
  },
  {
    id: "space-grotesk",
    name: "Space Grotesk 2.000 (Medium, Bold)",
    copyright: "Copyright 2020 The Space Grotesk Project Authors",
    license: "SIL Open Font License 1.1",
    files: ["SpaceGrotesk-Medium.woff2", "SpaceGrotesk-Bold.woff2"],
    text: spaceGroteskOfl,
  },
  {
    id: "jetbrains-mono",
    name: "JetBrains Mono 2.211",
    copyright: "Copyright 2020 The JetBrains Mono Project Authors",
    license: "SIL Open Font License 1.1",
    files: ["JetBrainsMono-latin.woff2", "JetBrainsMono-latin-ext.woff2"],
    text: jetBrainsMonoOfl,
  },
  {
    id: "lucide",
    name: "Lucide icons 1.48.0",
    copyright:
      "Copyright (c) 2026 Lucide Icons and Contributors; icons derived from Feather: Copyright (c) 2013-present Cole Bemis",
    license: "ISC License, MIT License (Feather-derived icons)",
    files: [],
    text: lucide,
  },
];
