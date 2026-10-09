# Projekte

Ein Projekt enthält eine Zeichnung, ihre Merkmale und Ballone sowie das Protokoll aller
Änderungen. Es wird als eine `.dimo`-Datei gespeichert. Die Zeichnung liegt unverändert in der
Datei, darum öffnet sich das Projekt auch dann, wenn das Original-PDF verschoben oder gelöscht
wurde.

## Anlegen, öffnen, speichern

| Aktion | Werkzeugleiste | macOS | Windows und Linux |
|---|---|---|---|
| Neues Projekt aus einer PDF-Zeichnung | Neues Projekt | ⌘N | Ctrl+N |
| Projekt öffnen | Projekt öffnen | ⌘O | Ctrl+O |
| Speichern | Speichern | ⌘S | Ctrl+S |
| Unter neuem Namen speichern | Speichern unter | ⇧⌘S | Ctrl+Shift+S |
| Rückgängig | ↶ | ⌘Z | Ctrl+Z |
| Wiederholen | ↷ | ⇧⌘Z | Ctrl+Shift+Z oder Ctrl+Y |

Ein neues Projekt hat bis zum ersten Speichern keine Datei; die Werkzeugleiste zeigt
"Unbenannt". Beim ersten Speichern fragt Dimo nach einem Dateinamen. Rückgängig ist unbegrenzt,
solange das Projekt offen ist; nach erneutem Öffnen beginnt der Verlauf neu.

Hat ein Projekt ungespeicherte Änderungen, fragt Dimo vor dem Anlegen oder Öffnen eines anderen
Projekts und vor dem Schließen des Fensters: speichern, nicht speichern oder abbrechen.

## Automatische Sicherung und Wiederherstellung

Jede Änderung wird sofort, spätestens aber alle 30 Sekunden, in ein Sicherungsjournal
geschrieben. Die Werkzeugleiste zeigt den Zustand: "Gespeichert", "Ungespeicherte Änderungen,
automatisch gesichert" oder eine Warnung, wenn das Journal nicht geschrieben werden konnte.

- Ein gespeichertes Projekt hat sein Journal neben der Datei: `part.dimo.journal`. Beim Speichern
  wird es in die Projektdatei übernommen und gelöscht.
- Ein noch nie gespeichertes Projekt hat sein Journal im Datenordner von Dimo (unter macOS zum
  Beispiel `~/Library/Application Support/io.github.nc-pb.dimo/autosave`).

Endet Dimo oder der Computer unerwartet, gehen höchstens die letzten 30 Sekunden verloren:

- Beim erneuten Öffnen des gespeicherten Projekts werden die Änderungen aus dem Journal
  wiederhergestellt.
- Ein noch nie gespeichertes Projekt wird beim nächsten Start von Dimo wiederhergestellt.

Ein Hinweis über der Zeichnung sagt, was wiederhergestellt wurde. Speichere das Projekt, um die
Änderungen zu behalten. Wählst du beim Schließen "Nicht speichern", wird das Journal gelöscht.

## Ein Fenster pro Projekt

Solange ein Projekt offen ist, legt Dimo daneben eine Sperrdatei an (`part.dimo.lock`). Ein
zweites Dimo-Fenster, das dasselbe Projekt öffnen will, erhält eine Meldung mit dem Benutzer, der
es geöffnet hat. Die Sperre endet, wenn Dimo schließt, auch nach einem Absturz; eine
liegengebliebene Sperrdatei blockiert also nie. Die Sperre gilt nur zwischen Dimo-Fenstern;
andere Programme können die Datei weiterhin ändern.
